/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, format_err, Context, Result};
use async_trait::async_trait;
use futures::future::BoxFuture;
use futures::{FutureExt, TryFutureExt};
use parking_lot::Mutex;
use rand::Rng;
use tokio::sync::{Notify, RwLock};

use cloned::cloned;
use futures_ext::future::{spawn_controlled, ControlledHandle, FbTryFutureExt, TryShared};
use futures_stats::TimedFutureExt;
use stats::prelude::*;

use bookmarks::{BookmarkName, Bookmarks};
use changeset_fetcher::ChangesetFetcher;
use context::CoreContext;
use mononoke_types::{ChangesetId, RepositoryId};

use crate::idmap::IdMap;
use crate::read_only::ReadOnlySegmentedChangelog;
use crate::update::{prepare_incremental_iddag_update, update_iddag};
use crate::{
    segmented_changelog_delegate, CloneData, Group, InProcessIdDag, Location, MismatchedHeadsError,
    SegmentedChangelog, StreamCloneData,
};

define_stats! {
    prefix = "mononoke.segmented_changelog.ondemand";
    location_to_changeset_id: timeseries(Sum),
    changeset_id_to_location: timeseries(Sum),
    missing_notification_handle: timeseries(Sum),
}

mod need_update {
    use stats::prelude::*;
    // The stats that are not per repo could be described as redundant. In situations where we are
    // debugging however it's nice to have the aggregation ready available to diagnose instances or
    // regions.
    define_stats! {
        prefix = "mononoke.segmented_changelog.need_update";
        count: timeseries(Sum),
        tries:
            histogram(1, 0, 30, Average, Sum, Count; P 5; P 25; P 50; P 75; P 95; P 97; P 99),
        duration_ms:
            histogram(1000, 0, 60_000, Average, Sum, Count; P 5; P 25; P 50; P 75; P 95; P 97; P 99),
        count_per_repo: dynamic_timeseries("{}.count", (repo_id: i32); Sum),
        tries_per_repo: dynamic_histogram(
            "{}.tries", (repo_id: i32);
            1, 0, 30, Average, Sum, Count; P 5; P 25; P 50; P 75; P 95; P 97; P 99
        ),
        duration_ms_per_repo: dynamic_histogram(
            "{}.duration_ms", (repo_id: i32);
            1000, 0, 60_000, Average, Sum, Count; P 5; P 25; P 50; P 75; P 95; P 97; P 99
        ),
    }
}

mod actual_update {
    use stats::prelude::*;
    define_stats! {
        prefix = "mononoke.segmented_changelog.update";
        count: timeseries(Sum),
        failure: timeseries(Sum),
        success: timeseries(Sum),
        duration_ms:
            histogram(1000, 0, 60_000, Average, Sum, Count; P 5; P 25; P 50; P 75; P 95; P 97; P 99),
        count_per_repo: dynamic_timeseries("{}.count", (repo_id: i32); Sum),
        failure_per_repo: dynamic_timeseries("{}.failure", (repo_id: i32); Sum),
        success_per_repo: dynamic_timeseries("{}.success", (repo_id: i32); Sum),
        duration_ms_per_repo: dynamic_histogram(
            "{}.duration_ms", (repo_id: i32);
            1000, 0, 60_000, Average, Sum, Count; P 5; P 25; P 50; P 75; P 95; P 97; P 99
        ),
    }
}

pub struct OnDemandUpdateSegmentedChangelog {
    repo_id: RepositoryId,
    iddag: Arc<RwLock<InProcessIdDag>>,
    idmap: Arc<dyn IdMap>,
    changeset_fetcher: Arc<dyn ChangesetFetcher>,
    bookmarks: Arc<dyn Bookmarks>,
    master_bookmark: BookmarkName,
    ongoing_update: Arc<Mutex<Option<TryShared<BoxFuture<'static, Result<()>>>>>>,
}

impl OnDemandUpdateSegmentedChangelog {
    pub fn new(
        repo_id: RepositoryId,
        iddag: InProcessIdDag,
        idmap: Arc<dyn IdMap>,
        changeset_fetcher: Arc<dyn ChangesetFetcher>,
        bookmarks: Arc<dyn Bookmarks>,
        master_bookmark: BookmarkName,
    ) -> Self {
        Self {
            repo_id,
            iddag: Arc::new(RwLock::new(iddag)),
            idmap,
            changeset_fetcher,
            bookmarks,
            master_bookmark,
            ongoing_update: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_periodic_update_to_master_bookmark(
        self: Arc<Self>,
        ctx: &CoreContext,
        period: Duration,
    ) -> PeriodicUpdateSegmentedChangelog {
        PeriodicUpdateSegmentedChangelog::new(ctx, self, period)
    }

    // Updating the Dag has 3 phases:
    // * loading the data that is required for the update;
    // * updating the IdMap;
    // * updating the IdDag;
    //
    // The Dag can function well for serving requests as long as the commits involved have been
    // built so we want to have easy read access to both the IdMap and the IdDag. The IdMap is a
    // very simple structure and because it's described as an Arc<dyn IdMap> we push the update
    // locking logic to the storage.  The IdDag is a complicated structure that we ask to update
    // itself. Those functions take mutable references. Updating the storage of the iddag to hide
    // the complexities of locking is more difficult. We deal with the IdDag directly by wrapping
    // it in a RwLock. The RwLock allows for easy read access which we expect to be the predominant
    // access pattern.
    //
    // Updates to the dag are not completely stable so racing updates can have conflicting results.
    // In case of conflics one of the update processes would have to restart. It's easier to reason
    // about the process if we just allow one "thread" to start an update process. The update
    // process is locked by a sync mutex. The "threads" that fail the race to update are asked to
    // wait until the ongoing update is complete. The waiters will poll on a shared future that
    // tracks the ongoing dag update. After the update is complete the waiters will go back to
    // checking if the data they have is available in the dag. It is possible that the dag is
    // updated in between determining that the an update is needed and acquiring the ongoing_update
    // lock. This is fine because the update building process checks the state of dag before the
    // dag and updates only what is necessary if necessary.
    /// Update the Dag to incorporate the commit pointed to by head.
    /// Returns true if it performed an actual update false if it simply waited for ongoing update
    /// to finish.
    async fn try_update(&self, ctx: &CoreContext, head: ChangesetId) -> Result<bool> {
        let to_wait = {
            let mut ongoing_update = self.ongoing_update.lock();

            if let Some(fut) = &*ongoing_update {
                fut.clone().map(|_| Ok(false)).boxed()
            } else {
                cloned!(
                    ctx,
                    self.repo_id,
                    self.iddag,
                    self.idmap,
                    self.changeset_fetcher
                );
                let task_ongoing_update = self.ongoing_update.clone();
                let update_task = async move {
                    let result =
                        the_actual_update(ctx, repo_id, iddag, idmap, changeset_fetcher, head)
                            .await;
                    let mut ongoing_update = task_ongoing_update.lock();
                    *ongoing_update = None;
                    result
                }
                .boxed()
                .try_shared();

                *ongoing_update = Some(update_task.clone());

                update_task.map_ok(|_| true).boxed()
            }
        };
        Ok(to_wait.await?)
    }

    // This functions will take a parameter, Group. It generalizes between updating the dag without
    // concerning itself with the group that the cs goes in. The functions that call into it have
    // the resposibility of deciding the Group.
    async fn build_up_to_cs(
        &self,
        ctx: &CoreContext,
        cs_id: ChangesetId,
        group: Group,
    ) -> Result<()> {
        if group != Group::MASTER {
            bail!("building the NON_MASTER group is not implemented for SegmentedChangelog");
        }
        let update_loop = async {
            let mut tries: i64 = 0;
            loop {
                tries += 1;
                if self.try_update(ctx, cs_id).await? {
                    return Ok(tries);
                }
                if self.is_cs_assigned(ctx, cs_id).await? {
                    return Ok(tries);
                }
            }
        };
        let (stats, ret) = update_loop.timed().await;
        match ret {
            Ok(tries) if tries > 0 => {
                need_update::STATS::count.add_value(1);
                need_update::STATS::count_per_repo.add_value(1, (self.repo_id.id(),));

                need_update::STATS::tries.add_value(tries);
                need_update::STATS::tries_per_repo.add_value(tries, (self.repo_id.id(),));

                need_update::STATS::duration_ms.add_value(stats.completion_time.as_millis() as i64);
                need_update::STATS::duration_ms_per_repo.add_value(
                    stats.completion_time.as_millis() as i64,
                    (self.repo_id.id(),),
                );
                let mut scuba = ctx.scuba().clone();
                scuba.add_future_stats(&stats);
                scuba.add("repo_id", self.repo_id.id());
                scuba.add("changeset_id", format!("{}", cs_id));
                scuba.add("tries", tries);
                scuba.log_with_msg("segmented_changelog_need_update", None);
            }
            _ => {}
        }
        ret.map(|_| ())
    }

    async fn build_up_to_bookmark(&self, ctx: &CoreContext) -> Result<()> {
        let bookmark_cs = self
            .bookmarks
            .get(ctx.clone(), &self.master_bookmark)
            .await
            .with_context(|| {
                format!(
                    "error while fetching changeset for bookmark {}",
                    self.master_bookmark
                )
            })?
            .ok_or_else(|| format_err!("'{}' bookmark could not be found", self.master_bookmark))?;
        self.build_up_to_cs(&ctx, bookmark_cs, Group::MASTER).await
    }

    async fn build_up_to_heads(&self, ctx: &CoreContext, heads: &[ChangesetId]) -> Result<()> {
        if !self.are_heads_assigned(ctx, heads).await? {
            self.build_up_to_bookmark(ctx).await?;
            // The IdDag has two groups, the MASTER group and the NON_MASTER group. The MASTER
            // group is reserved for the ancestors of the master bookmark. The MASTER group should
            // contain the ancestors of master. The NON_MASTER group can contain all other
            // changesets. When we don't know whether head is an ancestor of master, we need to
            // update to master first then we might assign head to the NonMaster group if we need
            // to. At the moment we only handle updating the MASTER group.
            // Note for the future. We should pay attention to potential races between a changeset
            // being used and the bookmark being updated.
            if !self.are_heads_assigned(ctx, heads).await? {
                let err = MismatchedHeadsError::new(self.repo_id, heads.to_vec());
                return Err(err.into());
            }
        }
        Ok(())
    }

    async fn are_heads_assigned(&self, ctx: &CoreContext, heads: &[ChangesetId]) -> Result<bool> {
        let dag_id_map = self.idmap.find_many_dag_ids(ctx, heads.to_vec()).await?;
        if dag_id_map.len() != heads.len() {
            // Maybe heads have duplicated items? Double check.
            let mut heads = heads.to_vec();
            heads.sort_unstable();
            heads.dedup();
            if dag_id_map.len() != heads.len() {
                return Ok(false);
            }
        }
        // It is safer to check that the dag_ids we got are also in the iddag.
        let iddag = self.iddag.read().await;
        for (_cs_id, dag_id) in dag_id_map {
            if !iddag.contains_id(dag_id)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn is_cs_assigned(&self, ctx: &CoreContext, cs_id: ChangesetId) -> Result<bool> {
        if let Some(dag_id) = self
            .idmap
            .find_dag_id(ctx, cs_id)
            .await
            .context("fetching dag_id for csid")?
        {
            // Note. This will result in two read locks being acquired for functions that call
            // into build_up. It would be nice to get to one lock being acquired. I tried but
            // had some issues with lifetimes :).
            let iddag = self.iddag.read().await;
            if iddag.contains_id(dag_id)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

async fn the_actual_update(
    ctx: CoreContext,
    repo_id: RepositoryId,
    iddag: Arc<RwLock<InProcessIdDag>>,
    idmap: Arc<dyn IdMap>,
    changeset_fetcher: Arc<dyn ChangesetFetcher>,
    head: ChangesetId,
) -> Result<()> {
    let monitored = async {
        let (head_dag_id, idmap_update_state) = {
            let iddag = iddag.read().await;
            prepare_incremental_iddag_update(&ctx, &iddag, &idmap, &changeset_fetcher, head)
                .await
                .context("error preparing an incremental update for iddag")?
        };
        if let Some((start_state, mem_idmap)) = idmap_update_state {
            let mut iddag = iddag.write().await;
            update_iddag(&ctx, &mut iddag, &start_state, &mem_idmap, head_dag_id)?;
        }
        Ok(())
    };
    actual_update::STATS::count.add_value(1);
    actual_update::STATS::count_per_repo.add_value(1, (repo_id.id(),));
    let (stats, ret) = monitored.timed().await;
    actual_update::STATS::duration_ms.add_value(stats.completion_time.as_millis() as i64);
    actual_update::STATS::duration_ms_per_repo
        .add_value(stats.completion_time.as_millis() as i64, (repo_id.id(),));
    if ret.is_ok() {
        actual_update::STATS::success.add_value(1);
        actual_update::STATS::success_per_repo.add_value(1, (repo_id.id(),));
    } else {
        actual_update::STATS::failure.add_value(1);
        actual_update::STATS::failure_per_repo.add_value(1, (repo_id.id(),));
    }
    let mut scuba = ctx.scuba().clone();
    scuba.add_future_stats(&stats);
    scuba.add("repo_id", repo_id.id());
    scuba.add("changeset_id", format!("{}", head));
    scuba.add("success", ret.is_ok());
    let msg = ret.as_ref().err().map(|err| format!("{:?}", err));
    scuba.log_with_msg("segmented_changelog_update", msg);
    ret
}

#[async_trait]
impl SegmentedChangelog for OnDemandUpdateSegmentedChangelog {
    async fn location_to_many_changeset_ids(
        &self,
        ctx: &CoreContext,
        location: Location<ChangesetId>,
        count: u64,
    ) -> Result<Vec<ChangesetId>> {
        STATS::location_to_changeset_id.add_value(1);
        // Location descendant may not be the ideal entry to build up to, it could be a good idea
        // to have client_head here too.
        self.build_up_to_heads(ctx, &[location.descendant])
            .await
            .context("error while getting an up to date dag")?;
        let iddag = self.iddag.read().await;
        let read_dag = ReadOnlySegmentedChangelog::new(&iddag, self.idmap.clone());
        read_dag
            .location_to_many_changeset_ids(ctx, location, count)
            .await
    }

    async fn many_changeset_ids_to_locations(
        &self,
        ctx: &CoreContext,
        master_heads: Vec<ChangesetId>,
        cs_ids: Vec<ChangesetId>,
    ) -> Result<HashMap<ChangesetId, Result<Location<ChangesetId>>>> {
        STATS::changeset_id_to_location.add_value(1);
        self.build_up_to_heads(ctx, &master_heads)
            .await
            .context("error while getting an up to date dag")?;
        let iddag = self.iddag.read().await;
        let read_dag = ReadOnlySegmentedChangelog::new(&iddag, self.idmap.clone());
        read_dag
            .many_changeset_ids_to_locations(ctx, master_heads, cs_ids)
            .await
    }

    async fn clone_data(&self, ctx: &CoreContext) -> Result<CloneData<ChangesetId>> {
        let iddag = self.iddag.read().await;
        let read_dag = ReadOnlySegmentedChangelog::new(&iddag, self.idmap.clone());
        read_dag.clone_data(ctx).await
    }

    async fn pull_fast_forward_master(
        &self,
        ctx: &CoreContext,
        old_master: ChangesetId,
        new_master: ChangesetId,
    ) -> Result<CloneData<ChangesetId>> {
        self.build_up_to_heads(ctx, &[old_master, new_master])
            .await
            .context("error while getting an up to date dag")?;
        let iddag = self.iddag.read().await;
        let read_dag = ReadOnlySegmentedChangelog::new(&iddag, self.idmap.clone());
        read_dag
            .pull_fast_forward_master(ctx, old_master, new_master)
            .await
    }

    async fn full_idmap_clone_data(
        &self,
        ctx: &CoreContext,
    ) -> Result<StreamCloneData<ChangesetId>> {
        let iddag = self.iddag.read().await;
        let read_dag = ReadOnlySegmentedChangelog::new(&iddag, self.idmap.clone());
        read_dag.full_idmap_clone_data(ctx).await
    }

    async fn disabled(&self, _ctx: &CoreContext) -> Result<bool> {
        Ok(false)
    }
    async fn is_ancestor(
        &self,
        ctx: &CoreContext,
        ancestor: ChangesetId,
        descendant: ChangesetId,
    ) -> Result<Option<bool>> {
        let iddag = self.iddag.read().await;
        let read_dag = ReadOnlySegmentedChangelog::new(&iddag, self.idmap.clone());
        read_dag.is_ancestor(ctx, ancestor, descendant).await
    }
}

pub struct PeriodicUpdateSegmentedChangelog {
    on_demand_update_sc: Arc<OnDemandUpdateSegmentedChangelog>,
    _handle: ControlledHandle,
    #[allow(dead_code)] // useful for testing
    notify: Arc<Notify>,
}

impl PeriodicUpdateSegmentedChangelog {
    pub fn new(
        ctx: &CoreContext,
        on_demand_update_sc: Arc<OnDemandUpdateSegmentedChangelog>,
        period: Duration,
    ) -> Self {
        let notify = Arc::new(Notify::new());
        let _handle = spawn_controlled({
            let ctx = ctx.clone();
            let my_dag = Arc::clone(&on_demand_update_sc);
            let notify = Arc::clone(&notify);
            async move {
                // jitter is here so not all repos try to update at the same time
                let jitter = rand::thread_rng().gen_range(Duration::from_secs(0), period);
                // there's lots of warmup happenning at server startup so wait at least a period
                // before starting to update
                tokio::time::sleep(period + jitter).await;

                let mut interval = tokio::time::interval(period);
                loop {
                    let _ = interval.tick().await;
                    if let Err(err) = my_dag.build_up_to_bookmark(&ctx).await {
                        slog::error!(
                            ctx.logger(),
                            "failed to update segmented changelog dag: {:?}",
                            err
                        );
                    }
                    notify.notify_waiters();
                }
            }
        });
        Self {
            on_demand_update_sc,
            _handle,
            notify,
        }
    }

    #[cfg(test)]
    pub async fn wait_for_update(&self) {
        self.notify.notified().await;
    }
}

segmented_changelog_delegate!(PeriodicUpdateSegmentedChangelog, |
    &self,
    ctx: &CoreContext,
| {
    &self.on_demand_update_sc
});
