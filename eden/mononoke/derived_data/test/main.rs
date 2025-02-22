/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{anyhow, Error, Result};
use async_trait::async_trait;
use blobstore::Blobstore;
use blobstore::{BlobstoreBytes, BlobstoreGetData};
use bookmarks::{BookmarkName, BookmarksRef};
use bytes::Bytes;
use cacheblob::LeaseOps;
use changesets::ChangesetsRef;
use cloned::cloned;
use context::CoreContext;
use fbinit::FacebookInit;
use fixtures::{
    branch_even, branch_uneven, branch_wide, linear, many_diamonds, many_files_dirs, merge_even,
    merge_uneven, unshared_merge_even, unshared_merge_uneven,
};
use futures::future::BoxFuture;
use futures_stats::{TimedFutureExt, TimedTryFutureExt};
use lock_ext::LockExt;
use maplit::hashmap;
use mononoke_types::{BonsaiChangeset, ChangesetId, MPath, RepositoryId};
use repo_blobstore::RepoBlobstoreRef;
use repo_derived_data::{RepoDerivedDataArc, RepoDerivedDataRef};
use test_repo_factory::TestRepoFactory;
use tests_utils::CreateCommitContext;
use tunables::{override_tunables, MononokeTunables};

use derived_data_manager::{dependencies, BonsaiDerivable, DerivationContext, DerivationError};

#[derive(Clone, Debug)]
struct DerivedGeneration {
    generation: u64,
}

impl From<DerivedGeneration> for BlobstoreBytes {
    fn from(derived: DerivedGeneration) -> BlobstoreBytes {
        let generation = derived.generation.to_string();
        let data = Bytes::copy_from_slice(generation.as_bytes());
        BlobstoreBytes::from_bytes(data)
    }
}

impl TryFrom<BlobstoreBytes> for DerivedGeneration {
    type Error = Error;

    fn try_from(blob_bytes: BlobstoreBytes) -> Result<Self> {
        let generation = std::str::from_utf8(blob_bytes.as_bytes())?.parse::<u64>()?;
        Ok(DerivedGeneration { generation })
    }
}

impl TryFrom<BlobstoreGetData> for DerivedGeneration {
    type Error = Error;

    fn try_from(data: BlobstoreGetData) -> Result<Self> {
        data.into_bytes().try_into()
    }
}

#[async_trait]
impl BonsaiDerivable for DerivedGeneration {
    const NAME: &'static str = "test_generation";

    type Dependencies = dependencies![];

    async fn derive_single(
        _ctx: &CoreContext,
        _derivation_ctx: &DerivationContext,
        bonsai: BonsaiChangeset,
        parents: Vec<Self>,
    ) -> Result<Self> {
        if let Some(delay_str) = bonsai
            .extra()
            .collect::<HashMap<_, _>>()
            .get("test-derive-delay")
        {
            let delay = std::str::from_utf8(delay_str)?.parse::<f64>()?;
            tokio::time::sleep(Duration::from_secs_f64(delay)).await;
        }
        let mut generation = 1;
        for parent in parents {
            if parent.generation >= generation {
                generation = parent.generation + 1;
            }
        }
        let derived = DerivedGeneration { generation };
        Ok(derived)
    }

    async fn store_mapping(
        self,
        ctx: &CoreContext,
        derivation_ctx: &DerivationContext,
        changeset_id: ChangesetId,
    ) -> Result<()> {
        derivation_ctx
            .blobstore()
            .put(
                ctx,
                format!(
                    "repo{}.test_generation.{}",
                    derivation_ctx.repo_id(),
                    changeset_id,
                ),
                self.into(),
            )
            .await?;
        Ok(())
    }

    async fn fetch(
        ctx: &CoreContext,
        derivation_ctx: &DerivationContext,
        changeset_id: ChangesetId,
    ) -> Result<Option<Self>> {
        match derivation_ctx
            .blobstore()
            .get(
                ctx,
                &format!(
                    "repo{}.test_generation.{}",
                    derivation_ctx.repo_id(),
                    changeset_id
                ),
            )
            .await?
        {
            Some(blob) => Ok(Some(blob.try_into()?)),
            None => Ok(None),
        }
    }
}

async fn derive_for_master(
    ctx: &CoreContext,
    repo: &(impl BookmarksRef + ChangesetsRef + RepoDerivedDataRef),
) -> Result<()> {
    let master = repo
        .bookmarks()
        .get(ctx.clone(), &BookmarkName::new("master")?)
        .await?
        .expect("master should be set");
    let expected = repo
        .changesets()
        .get(ctx.clone(), master)
        .await?
        .expect("changeset should exist")
        .gen;

    let derived = repo
        .repo_derived_data()
        .derive::<DerivedGeneration>(ctx, master)
        .await?;

    assert_eq!(expected, derived.generation);

    Ok(())
}

fn make_test_repo_factory() -> TestRepoFactory {
    let mut factory = TestRepoFactory::new().unwrap();
    factory.with_config_override(|repo_config| {
        repo_config
            .derived_data_config
            .enabled
            .types
            .insert(DerivedGeneration::NAME.to_string());
    });
    factory
}

#[fbinit::test]
async fn test_derive_fixtures(fb: FacebookInit) -> Result<()> {
    let ctx = CoreContext::test_mock(fb);
    let mut factory = make_test_repo_factory();

    let repo = factory.with_id(RepositoryId::new(1)).build()?;
    branch_even::initrepo(fb, &repo).await;
    derive_for_master(&ctx, &repo).await?;

    let repo = factory.with_id(RepositoryId::new(2)).build()?;
    branch_uneven::initrepo(fb, &repo).await;
    derive_for_master(&ctx, &repo).await?;

    let repo = factory.with_id(RepositoryId::new(3)).build()?;
    branch_wide::initrepo(fb, &repo).await;
    derive_for_master(&ctx, &repo).await?;

    let repo = factory.with_id(RepositoryId::new(4)).build()?;
    linear::initrepo(fb, &repo).await;
    derive_for_master(&ctx, &repo).await?;

    let repo = factory.with_id(RepositoryId::new(5)).build()?;
    many_files_dirs::initrepo(fb, &repo).await;
    derive_for_master(&ctx, &repo).await?;

    let repo = factory.with_id(RepositoryId::new(6)).build()?;
    merge_even::initrepo(fb, &repo).await;
    derive_for_master(&ctx, &repo).await?;

    let repo = factory.with_id(RepositoryId::new(7)).build()?;
    merge_uneven::initrepo(fb, &repo).await;
    derive_for_master(&ctx, &repo).await?;

    let repo = factory.with_id(RepositoryId::new(8)).build()?;
    unshared_merge_even::initrepo(fb, &repo).await;
    derive_for_master(&ctx, &repo).await?;

    let repo = factory.with_id(RepositoryId::new(9)).build()?;
    unshared_merge_uneven::initrepo(fb, &repo).await;
    derive_for_master(&ctx, &repo).await?;

    let repo = factory.with_id(RepositoryId::new(10)).build()?;
    many_diamonds::initrepo(fb, &repo).await;
    derive_for_master(&ctx, &repo).await?;

    Ok(())
}

#[fbinit::test]
/// Test that derivation is successful even when there are gaps (i.e. some
/// derived changesets do not have their parents derived).
async fn test_gapped_derivation(fb: FacebookInit) -> Result<()> {
    let ctx = CoreContext::test_mock(fb);
    let repo = make_test_repo_factory().build()?;
    linear::initrepo(fb, &repo).await;

    let master = repo
        .bookmarks()
        .get(ctx.clone(), &BookmarkName::new("master")?)
        .await?
        .expect("master should be set");
    let master_anc1 = repo
        .changesets()
        .get(ctx.clone(), master)
        .await?
        .expect("changeset should exist")
        .parents
        .first()
        .expect("changeset should have a parent")
        .clone();
    let master_anc2 = repo
        .changesets()
        .get(ctx.clone(), master_anc1)
        .await?
        .expect("changeset should exist")
        .parents
        .first()
        .expect("changeset should have a parent")
        .clone();
    // Insert a derived entry for the first ancestor changeset.  We will
    // deliberately use a different value than is expected.
    repo.repo_blobstore()
        .put(
            &ctx,
            format!("repo0.test_generation.{}", master_anc1),
            BlobstoreBytes::from_bytes(Bytes::from_static(b"41")),
        )
        .await?;

    // Derivation was based on that different value.
    let derived = repo
        .repo_derived_data()
        .derive::<DerivedGeneration>(&ctx, master)
        .await?;

    assert_eq!(42, derived.generation);

    // The other ancestors were not derived.
    let derived_anc2 = repo
        .repo_derived_data()
        .fetch_derived::<DerivedGeneration>(&ctx, master_anc2)
        .await?;
    assert!(derived_anc2.is_none());

    Ok(())
}

#[fbinit::test]
async fn test_leases(fb: FacebookInit) -> Result<(), Error> {
    let ctx = CoreContext::test_mock(fb);
    let repo = make_test_repo_factory().build()?;
    linear::initrepo(fb, &repo).await;

    let master = repo
        .bookmarks()
        .get(ctx.clone(), &BookmarkName::new("master")?)
        .await?
        .expect("master should be set");

    let lease = repo.repo_derived_data().lease();
    let lease_key = format!(
        "repo{}.{}.{}",
        repo.get_repoid().id(),
        DerivedGeneration::NAME,
        master
    );

    // take lease
    assert_eq!(lease.try_add_put_lease(&lease_key).await?, true);
    assert_eq!(lease.try_add_put_lease(&lease_key).await?, false);

    let output = Arc::new(Mutex::new(None));
    tokio::spawn({
        cloned!(ctx, repo, output);
        async move {
            let result = repo
                .repo_derived_data()
                .derive::<DerivedGeneration>(&ctx, master)
                .await;
            output.with(move |output| {
                let _ = output.insert(result);
            });
        }
    });

    // Let the derivation process get started, however derivation won't
    // happen yet.
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert!(
        repo.repo_derived_data()
            .fetch_derived::<DerivedGeneration>(&ctx, master)
            .await?
            .is_none()
    );

    // Release the lease, allowing derivation to proceed.
    lease.release_lease(&lease_key).await;
    tokio::time::sleep(Duration::from_millis(3000)).await;

    let expected = repo
        .changesets()
        .get(ctx.clone(), master)
        .await?
        .expect("changeset should exist")
        .gen;
    let result = output
        .with(|output| output.take())
        .expect("scheduled derivation should have completed")?;
    assert_eq!(expected, result.generation,);

    // Take the lease again.
    assert_eq!(lease.try_add_put_lease(&lease_key).await?, true);

    // This time it should succeed, as the lease won't be necessary.
    let result = repo
        .repo_derived_data()
        .derive::<DerivedGeneration>(&ctx, master)
        .await?;
    assert_eq!(expected, result.generation);
    lease.release_lease(&lease_key).await;
    Ok(())
}

#[derive(Debug)]
struct FailingLease;

impl std::fmt::Display for FailingLease {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "FailingLease")
    }
}

#[async_trait]
impl LeaseOps for FailingLease {
    async fn try_add_put_lease(&self, _key: &str) -> Result<bool> {
        Err(anyhow!("error"))
    }

    fn renew_lease_until(&self, _ctx: CoreContext, _key: &str, _done: BoxFuture<'static, ()>) {}

    async fn wait_for_other_leases(&self, _key: &str) {}

    async fn release_lease(&self, _key: &str) {}
}

#[fbinit::test]
async fn test_always_failing_lease(fb: FacebookInit) -> Result<(), Error> {
    let ctx = CoreContext::test_mock(fb);
    let repo = make_test_repo_factory()
        .with_derived_data_lease(|| Arc::new(FailingLease))
        .build()?;
    linear::initrepo(fb, &repo).await;

    let master = repo
        .bookmarks()
        .get(ctx.clone(), &BookmarkName::new("master")?)
        .await?
        .expect("master should be set");

    let lease = repo.repo_derived_data().lease();
    let lease_key = format!(
        "repo{}.{}.{}",
        repo.get_repoid().id(),
        DerivedGeneration::NAME,
        master,
    );

    // Taking the lease should fail
    assert!(lease.try_add_put_lease(&lease_key).await.is_err());

    // Derivation should succeed even though lease always fails
    let result = repo
        .repo_derived_data()
        .derive::<DerivedGeneration>(&ctx, master)
        .await?;
    let expected = repo
        .changesets()
        .get(ctx.clone(), master)
        .await?
        .expect("changeset should exist")
        .gen;
    assert_eq!(expected, result.generation);

    Ok(())
}

#[fbinit::test]
async fn test_parallel_derivation(fb: FacebookInit) -> Result<(), Error> {
    let ctx = CoreContext::test_mock(fb);
    let repo = make_test_repo_factory().build()?;

    // Create a commit with lots of parents, and make each derivation take
    // 2 seconds.  Check that derivations happen in parallel by ensuring
    // it completes in a shorter time.
    let mut parents = vec![];
    for i in 0..8 {
        let p = CreateCommitContext::new_root(&ctx, &repo)
            .add_file(MPath::new(format!("file_{}", i))?, format!("{}", i))
            .add_extra("test-derive-delay", "2")
            .commit()
            .await?;
        parents.push(p);
    }

    let merge = CreateCommitContext::new(&ctx, &repo, parents)
        .add_extra("test-derive-delay", "2")
        .commit()
        .await?;

    let (stats, _res) = repo
        .repo_derived_data()
        .derive::<DerivedGeneration>(&ctx, merge)
        .try_timed()
        .await?;

    assert!(stats.completion_time > Duration::from_secs(2));
    assert!(stats.completion_time < Duration::from_secs(10));

    Ok(())
}

async fn ensure_tunables_disable_derivation(
    ctx: &CoreContext,
    repo: &impl RepoDerivedDataArc,
    csid: ChangesetId,
    tunables: MononokeTunables,
) -> Result<(), Error> {
    let spawned_derivation = tokio::spawn({
        let ctx = ctx.clone();
        let repo_derived_data = repo.repo_derived_data_arc();
        async move {
            repo_derived_data
                .derive::<DerivedGeneration>(&ctx, csid)
                .timed()
                .await
        }
    });
    tokio::time::sleep(Duration::from_millis(1000)).await;

    override_tunables(Some(Arc::new(tunables)));

    let (stats, res) = spawned_derivation.await?;

    assert!(matches!(res, Err(DerivationError::Disabled(..))));

    eprintln!("derivation cancelled after {:?}", stats.completion_time);
    assert!(stats.completion_time < Duration::from_secs(15));

    Ok(())
}

#[fbinit::test]
/// Test that very slow derivation can be cancelled mid-flight by setting
/// the appropriate values in tunables.
async fn test_cancelling_slow_derivation(fb: FacebookInit) -> Result<(), Error> {
    let ctx = CoreContext::test_mock(fb);
    let repo = make_test_repo_factory().build()?;

    let create_tunables = || {
        let tunables = MononokeTunables::default();
        // Make delay smaller so that the test runs faster
        tunables.update_ints(&hashmap! {
            "derived_data_disabled_watcher_delay_secs".to_string() => 1,
        });
        tunables
    };

    let commit = CreateCommitContext::new_root(&ctx, &repo)
        .add_file(MPath::new("file")?, "content")
        .add_extra("test-derive-delay", "20")
        .commit()
        .await?;

    // Reset tunables.
    override_tunables(Some(Arc::new(create_tunables())));

    // Disable derived data for all types
    let tunables_to_disable_all = create_tunables();
    tunables_to_disable_all.update_by_repo_bools(&hashmap! {
        repo.name().to_string() => hashmap! {
            "all_derived_data_disabled".to_string() => true,
        },
    });
    ensure_tunables_disable_derivation(&ctx, &repo, commit, tunables_to_disable_all).await?;

    // Reset tunables.
    override_tunables(Some(Arc::new(create_tunables())));

    // Disable derived data for a single type
    let tunables_to_disable_by_type = create_tunables();
    tunables_to_disable_by_type.update_by_repo_vec_of_strings(&hashmap! {
        repo.name().to_string() => hashmap! {
            "derived_data_types_disabled".to_string() => vec![DerivedGeneration::NAME.to_string()],
        },
    });
    ensure_tunables_disable_derivation(&ctx, &repo, commit, tunables_to_disable_by_type).await?;

    Ok(())
}
