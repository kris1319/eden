/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#![deny(warnings)]

use std::vec::Vec;

use anyhow::Error;
use bytes::Bytes;
use futures::future::{BoxFuture, FutureExt};
use sql::{queries, Connection};
use sql_construct::{SqlConstruct, SqlConstructFromMetadataDatabaseConfig};
use sql_ext::SqlConnections;
use thiserror::Error;

use blobstore::Blobstore;
use context::{CoreContext, PerfCounterType};
use mononoke_types::RepositoryId;

#[derive(Debug, Error)]
pub enum ErrorKind {
    #[error("missing blob {0}")]
    MissingStreamingBlob(String),
    #[error("incorrect size {1} (expected {2}) of corrupt blob {0}")]
    CorruptStreamingBlob(String, usize, usize),
}

pub struct RevlogStreamingChunks {
    pub index_size: usize,
    pub data_size: usize,
    pub index_blobs: Vec<BoxFuture<'static, Result<Bytes, Error>>>,
    pub data_blobs: Vec<BoxFuture<'static, Result<Bytes, Error>>>,
}

impl RevlogStreamingChunks {
    pub fn new() -> Self {
        Self {
            data_size: 0,
            index_size: 0,
            data_blobs: Vec::new(),
            index_blobs: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct SqlStreamingChunksFetcher {
    read_connection: Connection,
    write_connection: Connection,
}

queries! {
    read CountChunks(repo_id: RepositoryId, tag: &str) -> (u64) {
        "SELECT count(*)
         FROM streaming_changelog_chunks
         WHERE repo_id = {repo_id} and tag = {tag}"
    }
    read SelectChunks(repo_id: RepositoryId, tag: &str) -> (Vec<u8>, i32, Vec<u8>, i32) {
        "SELECT idx_blob_name, idx_size, data_blob_name, data_size
         FROM streaming_changelog_chunks
         WHERE repo_id = {repo_id} and tag = {tag}
         ORDER BY chunk_num ASC"
    }

    read SelectSizes(repo_id: RepositoryId, tag: &str) -> (Option<u64>, Option<u64>) {
        "SELECT CAST(SUM(idx_size) AS UNSIGNED), CAST(SUM(data_size) AS UNSIGNED)
         FROM streaming_changelog_chunks
         WHERE repo_id = {repo_id} and tag = {tag}"
    }

    write InsertChunks(
        values: (
            repo_id: RepositoryId,
            tag: &str,
            chunk_num: u32,
            idx_blob_name: &str,
            idx_size: u32,
            data_blob_name: &str,
            data_size: u32,
        )
    ) {
        none,
        "INSERT INTO streaming_changelog_chunks \
            (repo_id, tag, chunk_num, idx_blob_name, idx_size, data_blob_name, data_size) \
            VALUES {values}"
    }

    read SelectMaxChunkNum(repo_id: RepositoryId) -> (Option<u32>) {
        "SELECT max(chunk_num)
         FROM streaming_changelog_chunks
         WHERE repo_id = {repo_id}"
    }
}

impl SqlConstruct for SqlStreamingChunksFetcher {
    const LABEL: &'static str = "streaming-chunks";

    const CREATION_QUERY: &'static str = "
        CREATE TABLE `streaming_changelog_chunks` (
        `repo_id` int(11) NOT NULL,
        `tag` varbinary(100) NOT NULL DEFAULT '',
        `chunk_num` int(11) NOT NULL,
        `idx_blob_name` varbinary(4096) NOT NULL,
        `idx_size` int(11) NOT NULL,
        `data_blob_name` varbinary(4096) NOT NULL,
        `data_size` int(11) NOT NULL,
        PRIMARY KEY (`repo_id`,`tag`,`chunk_num`)
        )
    ";

    fn from_sql_connections(connections: SqlConnections) -> Self {
        Self {
            read_connection: connections.read_connection,
            write_connection: connections.write_connection,
        }
    }
}

impl SqlConstructFromMetadataDatabaseConfig for SqlStreamingChunksFetcher {}

fn fetch_blob(
    ctx: CoreContext,
    blobstore: impl Blobstore + 'static,
    key: &[u8],
    expected_size: usize,
) -> BoxFuture<'static, Result<Bytes, Error>> {
    let key = String::from_utf8_lossy(key).into_owned();

    async move {
        let data = blobstore.get(&ctx, &key).await?;

        match data {
            None => Err(ErrorKind::MissingStreamingBlob(key).into()),
            Some(data) if data.as_bytes().len() == expected_size => Ok(data.into_raw_bytes()),
            Some(data) => {
                Err(
                    ErrorKind::CorruptStreamingBlob(key, data.as_bytes().len(), expected_size)
                        .into(),
                )
            }
        }
    }
    .boxed()
}

impl SqlStreamingChunksFetcher {
    pub async fn count_chunks(
        &self,
        ctx: &CoreContext,
        repo_id: RepositoryId,
        tag: Option<&str>,
    ) -> Result<u64, Error> {
        ctx.perf_counters()
            .increment_counter(PerfCounterType::SqlReadsReplica);

        let tag = tag.unwrap_or("");
        let res = CountChunks::query(&self.read_connection, &repo_id, &tag).await?;
        Ok(res.get(0).map_or(0, |x| x.0))
    }

    pub async fn fetch_changelog(
        &self,
        ctx: CoreContext,
        repo_id: RepositoryId,
        tag: Option<&str>,
        blobstore: impl Blobstore + Clone + 'static,
    ) -> Result<RevlogStreamingChunks, Error> {
        ctx.perf_counters()
            .increment_counter(PerfCounterType::SqlReadsReplica);

        let tag = tag.unwrap_or("");
        let rows = SelectChunks::query(&self.read_connection, &repo_id, &tag).await?;

        let res = rows.into_iter().fold(
            RevlogStreamingChunks::new(),
            move |mut res, (idx_blob_name, idx_size, data_blob_name, data_size)| {
                let data_size = data_size as usize;
                let idx_size = idx_size as usize;
                res.data_size += data_size;
                res.index_size += idx_size;
                res.data_blobs.push(fetch_blob(
                    ctx.clone(),
                    blobstore.clone(),
                    &data_blob_name,
                    data_size,
                ));
                res.index_blobs.push(fetch_blob(
                    ctx.clone(),
                    blobstore.clone(),
                    &idx_blob_name,
                    idx_size,
                ));
                res
            },
        );

        Ok(res)
    }

    pub async fn insert_chunks(
        &self,
        ctx: &CoreContext,
        repo_id: RepositoryId,
        tag: Option<&str>,
        chunks: Vec<(u32, &str, u32, &str, u32)>,
    ) -> Result<(), Error> {
        ctx.perf_counters()
            .increment_counter(PerfCounterType::SqlWrites);

        let tag = tag.unwrap_or("");

        let ref_chunks: Vec<_> = chunks
            .iter()
            .map(|row| (&repo_id, &tag, &row.0, &row.1, &row.2, &row.3, &row.4))
            .collect();

        InsertChunks::query(&self.write_connection, &ref_chunks[..]).await?;

        Ok(())
    }

    pub async fn select_index_and_data_sizes(
        &self,
        ctx: &CoreContext,
        repo_id: RepositoryId,
        tag: Option<&str>,
    ) -> Result<Option<(u64, u64)>, Error> {
        ctx.perf_counters()
            .increment_counter(PerfCounterType::SqlReadsReplica);

        let tag = tag.unwrap_or("");

        let res = SelectSizes::query(&self.read_connection, &repo_id, &tag).await?;
        let (idx, data) = match res.get(0) {
            Some((Some(idx), Some(data))) => (idx, data),
            _ => {
                return Ok(None);
            }
        };

        Ok(Some((*idx, *data)))
    }

    pub async fn select_max_chunk_num(
        &self,
        ctx: &CoreContext,
        repo_id: RepositoryId,
    ) -> Result<Option<u32>, Error> {
        ctx.perf_counters()
            .increment_counter(PerfCounterType::SqlReadsReplica);

        let res = SelectMaxChunkNum::query(&self.read_connection, &repo_id).await?;
        Ok(res.get(0).and_then(|x| x.0))
    }
}
