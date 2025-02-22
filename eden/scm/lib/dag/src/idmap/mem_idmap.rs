/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use super::IdMapWrite;
use crate::errors::NotFoundError;
use crate::id::{Group, Id, VertexName};
use crate::ops::IdConvert;
use crate::ops::Persist;
use crate::ops::PrefixLookup;
use crate::Result;
use crate::VerLink;
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{self, AtomicU64};

/// Bi-directional mapping between an integer id and a name (`[u8]`).
///
/// Private. Stored in memory.
pub struct MemIdMap {
    core: CoreMemIdMap,
    cached_next_free_ids: [AtomicU64; Group::COUNT],
    map_id: String,
    map_version: VerLink,
}

/// Subset of the `MemIdMap` interface that does not have "next_free_id"
/// or "version" concept.
#[derive(Default, Clone)]
pub(crate) struct CoreMemIdMap {
    id2name: HashMap<Id, VertexName>,
    name2id: BTreeMap<VertexName, Id>,
}

impl MemIdMap {
    /// Create an empty [`MemIdMap`].
    pub fn new() -> Self {
        Self {
            core: Default::default(),
            cached_next_free_ids: Default::default(),
            map_id: format!("mem:{}", next_id()),
            map_version: VerLink::new(),
        }
    }
}

impl Clone for MemIdMap {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            map_id: self.map_id.clone(),
            map_version: self.map_version.clone(),
            cached_next_free_ids: [
                AtomicU64::new(self.cached_next_free_ids[0].load(atomic::Ordering::SeqCst)),
                AtomicU64::new(self.cached_next_free_ids[1].load(atomic::Ordering::SeqCst)),
            ],
        }
    }
}

impl CoreMemIdMap {
    pub fn lookup_vertex_id(&self, name: &VertexName) -> Option<Id> {
        self.name2id.get(&name).copied()
    }

    pub fn lookup_vertex_name(&self, id: Id) -> Option<VertexName> {
        self.id2name.get(&id).cloned()
    }

    pub fn lookup_vertexes_by_hex_prefix(
        &self,
        hex_prefix: &[u8],
        limit: usize,
    ) -> Result<Vec<VertexName>> {
        let start = VertexName::from_hex(hex_prefix)?;
        let mut result = Vec::new();
        for (vertex, _) in self.name2id.range(start..) {
            if !vertex.to_hex().as_bytes().starts_with(hex_prefix) {
                break;
            }
            result.push(vertex.clone());
            if result.len() >= limit {
                break;
            }
        }
        Ok(result)
    }

    pub fn has_vertex_name(&self, name: &VertexName) -> bool {
        self.name2id.contains_key(name)
    }

    pub fn has_vertex_id(&self, id: Id) -> bool {
        self.id2name.contains_key(&id)
    }

    pub fn insert_vertex_id_name(&mut self, id: Id, vertex_name: VertexName) {
        self.name2id.insert(vertex_name.clone(), id);
        self.id2name.insert(id, vertex_name);
    }
}

#[async_trait::async_trait]
impl IdConvert for MemIdMap {
    async fn vertex_id(&self, name: VertexName) -> Result<Id> {
        self.core
            .lookup_vertex_id(&name)
            .ok_or_else(|| name.not_found_error())
    }
    async fn vertex_id_with_max_group(
        &self,
        name: &VertexName,
        max_group: Group,
    ) -> Result<Option<Id>> {
        let optional_id = self.core.name2id.get(name).and_then(|id| {
            if id.group() <= max_group {
                Some(*id)
            } else {
                None
            }
        });
        Ok(optional_id)
    }
    async fn vertex_name(&self, id: Id) -> Result<VertexName> {
        self.core
            .lookup_vertex_name(id)
            .ok_or_else(|| id.not_found_error())
    }
    async fn contains_vertex_name(&self, name: &VertexName) -> Result<bool> {
        Ok(self.core.has_vertex_name(name))
    }

    async fn contains_vertex_id_locally(&self, ids: &[Id]) -> Result<Vec<bool>> {
        Ok(ids
            .iter()
            .copied()
            .map(|id| self.core.has_vertex_id(id))
            .collect())
    }
    async fn contains_vertex_name_locally(&self, names: &[VertexName]) -> Result<Vec<bool>> {
        Ok(names
            .iter()
            .map(|name| self.core.has_vertex_name(name))
            .collect())
    }

    fn map_id(&self) -> &str {
        &self.map_id
    }

    fn map_version(&self) -> &VerLink {
        &self.map_version
    }
}

// TODO: Reconsider re-assign master cases. Currently they are ignored.
impl IdMapWrite for MemIdMap {
    fn insert(&mut self, id: Id, name: &[u8]) -> Result<()> {
        let vertex_name = VertexName::copy_from(name);
        self.core.insert_vertex_id_name(id, vertex_name);
        let group = id.group();
        // TODO: use fetch_max once stabilized.
        // (https://github.com/rust-lang/rust/issues/4865)
        let cached = self.cached_next_free_ids[group.0].load(atomic::Ordering::SeqCst);
        if id.0 >= cached {
            self.cached_next_free_ids[group.0].store(id.0 + 1, atomic::Ordering::SeqCst);
        }
        self.map_version.bump();
        Ok(())
    }
    fn next_free_id(&self, group: Group) -> Result<Id> {
        let cached = self.cached_next_free_ids[group.0].load(atomic::Ordering::SeqCst);
        let id = Id(cached).max(group.min_id());
        Ok(id)
    }
    fn remove_non_master(&mut self) -> Result<()> {
        self.map_version = VerLink::new();
        Ok(())
    }
    fn need_rebuild_non_master(&self) -> bool {
        false
    }
}

impl Persist for MemIdMap {
    type Lock = ();

    fn lock(&mut self) -> Result<Self::Lock> {
        Ok(())
    }

    fn reload(&mut self, _lock: &Self::Lock) -> Result<()> {
        Ok(())
    }

    fn persist(&mut self, _lock: &Self::Lock) -> Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl PrefixLookup for MemIdMap {
    async fn vertexes_by_hex_prefix(
        &self,
        hex_prefix: &[u8],
        limit: usize,
    ) -> Result<Vec<VertexName>> {
        self.core.lookup_vertexes_by_hex_prefix(hex_prefix, limit)
    }
}

fn next_id() -> u64 {
    static ID: AtomicU64 = AtomicU64::new(0);
    ID.fetch_add(1, atomic::Ordering::AcqRel)
}
