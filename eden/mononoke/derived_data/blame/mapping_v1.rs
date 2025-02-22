/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use anyhow::{anyhow, Error, Result};
use async_trait::async_trait;
use blobstore::{Blobstore, BlobstoreBytes};
use context::CoreContext;
use derived_data::impl_bonsai_derived_via_manager;
use derived_data_manager::{dependencies, BonsaiDerivable, DerivationContext};
use metaconfig_types::BlameVersion;
use mononoke_types::{BonsaiChangeset, ChangesetId};
use unodes::RootUnodeManifestId;

use crate::derive_v1::derive_blame_v1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlameRoot(ChangesetId);

impl From<ChangesetId> for BlameRoot {
    fn from(csid: ChangesetId) -> BlameRoot {
        BlameRoot(csid)
    }
}

fn format_key(changeset_id: ChangesetId) -> String {
    format!("derived_rootblame.v1.{}", changeset_id)
}

#[async_trait]
impl BonsaiDerivable for BlameRoot {
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
        if derivation_ctx.config().blame_version != BlameVersion::V1 {
            return Err(anyhow!(
                "programming error: incorrect blame version (expected V1)"
            ));
        }
        derive_blame_v1(ctx, derivation_ctx, bonsai, root_manifest).await?;
        Ok(BlameRoot(csid))
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
            .put(ctx, key, BlobstoreBytes::empty())
            .await
    }

    async fn fetch(
        ctx: &CoreContext,
        derivation_ctx: &DerivationContext,
        changeset_id: ChangesetId,
    ) -> Result<Option<Self>> {
        let key = format_key(changeset_id);
        match derivation_ctx.blobstore().get(ctx, &key).await? {
            Some(_) => Ok(Some(BlameRoot(changeset_id))),
            None => Ok(None),
        }
    }
}

impl_bonsai_derived_via_manager!(BlameRoot);
