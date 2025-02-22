/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use anyhow::{anyhow, Error, Result};
use async_trait::async_trait;
use blobstore::Blobstore;
use context::CoreContext;
use derived_data::impl_bonsai_derived_via_manager;
use derived_data_manager::{dependencies, BonsaiDerivable, DerivationContext};
use metaconfig_types::BlameVersion;
use mononoke_types::{BonsaiChangeset, ChangesetId};
use std::collections::HashMap;
use std::convert::TryInto;
use unodes::RootUnodeManifestId;

use crate::batch_v2::derive_blame_v2_in_batch;
use crate::derive_v2::derive_blame_v2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootBlameV2 {
    pub(crate) csid: ChangesetId,
    pub(crate) root_manifest: RootUnodeManifestId,
}

impl RootBlameV2 {
    pub fn root_manifest(&self) -> RootUnodeManifestId {
        self.root_manifest
    }
}

fn format_key(changeset_id: ChangesetId) -> String {
    format!("derived_root_blame_v2.{}", changeset_id)
}

#[async_trait]
impl BonsaiDerivable for RootBlameV2 {
    const NAME: &'static str = "blame";

    type Dependencies = dependencies![RootUnodeManifestId];

    async fn derive_single(
        ctx: &CoreContext,
        derivation_ctx: &DerivationContext,
        bonsai: BonsaiChangeset,
        _parents: Vec<Self>,
    ) -> Result<Self, Error> {
        let csid = bonsai.get_changeset_id();
        let root_manifest = derivation_ctx
            .derive_dependency::<RootUnodeManifestId>(ctx, csid)
            .await?;
        if derivation_ctx.config().blame_version != BlameVersion::V2 {
            return Err(anyhow!(
                "programming error: incorrect blame version (expected V2)"
            ));
        }
        derive_blame_v2(ctx, derivation_ctx, bonsai, root_manifest).await?;
        Ok(RootBlameV2 {
            csid,
            root_manifest,
        })
    }

    async fn derive_batch(
        ctx: &CoreContext,
        derivation_ctx: &DerivationContext,
        bonsais: Vec<BonsaiChangeset>,
        _gap_size: Option<usize>,
    ) -> Result<HashMap<ChangesetId, Self>, Error> {
        derive_blame_v2_in_batch(ctx, derivation_ctx, bonsais).await
    }

    async fn store_mapping(
        self,
        ctx: &CoreContext,
        derivation_ctx: &DerivationContext,
        changeset_id: ChangesetId,
    ) -> Result<()> {
        let key = format_key(changeset_id);
        derivation_ctx
            .blobstore()
            .put(ctx, key, self.root_manifest.into())
            .await
    }

    async fn fetch(
        ctx: &CoreContext,
        derivation_ctx: &DerivationContext,
        changeset_id: ChangesetId,
    ) -> Result<Option<Self>> {
        let key = format_key(changeset_id);
        match derivation_ctx.blobstore().get(ctx, &key).await? {
            Some(value) => Ok(Some(RootBlameV2 {
                csid: changeset_id,
                root_manifest: value.try_into()?,
            })),
            None => Ok(None),
        }
    }
}

impl_bonsai_derived_via_manager!(RootBlameV2);
