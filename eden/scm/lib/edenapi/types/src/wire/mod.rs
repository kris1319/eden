/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

//! This module contains wire representation structs for external types we'd
//! like to avoid explicitly depending on. Types will be added as they are
//! used.
//!
//! These types should all be `pub(crate)`. They're used extensively inside the
//! crate, but should never appear outside it. The methods on the request /
//! response objects should accept and return the public types from
//! `eden/scm/lib/types`.
//!
//! To maintain wire-protocol compatibility, we have some important conventions
//! and requirements for all types defined inside this module:
//!
//! 1. Every field should be renamed to a unique natural number using
//! `#[serde(rename = "0")]`. New fields should never re-use a field identifier
//! that has been used before. If a field changes semantically, it should be
//! considered a new field, and be given a new identifier.
//!
//! 2. Every enum should have an "Unknown" variant as the last variant in the
//! enum. This variant should be annotated with `#[serde(other, rename = "0")]`
//!
//! 3. When practical, fields should be annotated with
//! `#[serde(default, skip_serializing_if = "is_default")` to save space on the
//! wire. Do not use `#[serde(default)]` on the container.
//!
//! 4. All fields should be wrapped in `Option` or in a container that may be
//! empty, such as `Vec`. If an empty container has special semantics (other
//! than ignoring the field), please wrap that field in an `Option` as well to
//! distinguish between "empty" and "not present".
//!
//! Things to update when making a change to a wire type:
//!
//! 1. The Wire type definition.
//! 2. If applicable, the API type definition.
//! 3. The `ToWire` and `ToApi` implementations for the wire type.
//! 4. If the API type has changed, the `json` module.
//! 5. The `Arbitrary` implementations for the modified types.
//! 6. If a new type is introduced, add a quickcheck serialize round trip test.
//! 7. If the type has a corresponding API type, add a quickcheck wire-API round
//! trip test.

#[macro_use]
pub mod hash;

pub mod anyid;
pub mod batch;
pub mod bookmark;
pub mod clone;
pub mod commit;
pub mod complete_tree;
pub mod errors;
pub mod file;
pub mod history;
pub mod metadata;
pub mod pull;
pub mod token;
pub mod tree;

use dag_types::id::Id as DagId;

pub use crate::wire::{
    anyid::{WireAnyId, WireLookupRequest, WireLookupResponse},
    batch::WireBatch,
    bookmark::{WireBookmarkEntry, WireBookmarkRequest},
    clone::{WireCloneData, WireIdMapEntry},
    commit::{
        WireCommitGraphEntry, WireCommitGraphRequest, WireCommitHashLookupRequest,
        WireCommitHashLookupResponse, WireCommitHashToLocationRequestBatch,
        WireCommitHashToLocationResponse, WireCommitLocation, WireCommitLocationToHashRequest,
        WireCommitLocationToHashRequestBatch, WireCommitLocationToHashResponse,
        WireEphemeralPrepareRequest, WireEphemeralPrepareResponse, WireExtra,
        WireFetchSnapshotRequest, WireFetchSnapshotResponse, WireHgChangesetContent,
        WireHgMutationEntryContent, WireUploadBonsaiChangesetRequest, WireUploadHgChangeset,
        WireUploadHgChangesetsRequest,
    },
    complete_tree::WireCompleteTreeRequest,
    errors::{WireError, WireResult},
    file::{WireFileEntry, WireFileRequest, WireUploadHgFilenodeRequest, WireUploadTokensResponse},
    history::{WireHistoryRequest, WireHistoryResponseChunk, WireWireHistoryEntry},
    metadata::{
        WireAnyFileContentId, WireContentId, WireDirectoryMetadata, WireDirectoryMetadataRequest,
        WireFileMetadata, WireFileMetadataRequest, WireFileType, WireSha1, WireSha256,
    },
    token::{WireUploadToken, WireUploadTokenData, WireUploadTokenSignature},
    tree::{
        WireTreeEntry, WireTreeRequest, WireUploadTreeEntry, WireUploadTreeRequest,
        WireUploadTreeResponse,
    },
};

use std::convert::Infallible;
use std::fmt;

#[cfg(any(test, feature = "for-tests"))]
use quickcheck::Arbitrary;
use serde_derive::{Deserialize, Serialize};
use thiserror::Error;

use revisionstore_types::Metadata as RevisionstoreMetadata;
use types::{path::ParseError as RepoPathParseError, HgId, Key, Parents, RepoPathBuf};

use crate::EdenApiServerErrorKind;

#[derive(Copy, Clone, Debug, Error)]
#[error("invalid byte slice length, expected {expected_len} found {found_len}")]
pub struct TryFromBytesError {
    pub expected_len: usize,
    pub found_len: usize,
}

#[derive(Debug, Error)]
#[error("Failed to convert from wire to API representation")]
pub enum WireToApiConversionError {
    UnrecognizedEnumVariant(&'static str),
    CannotPopulateRequiredField(&'static str),
    PathValidationError(RepoPathParseError),
    InvalidUploadTokenType(&'static str),
}

impl From<Infallible> for WireToApiConversionError {
    fn from(v: Infallible) -> Self {
        match v {}
    }
}

impl From<RepoPathParseError> for WireToApiConversionError {
    fn from(v: RepoPathParseError) -> Self {
        WireToApiConversionError::PathValidationError(v)
    }
}

/// Convert from an EdenAPI API type to Wire type
pub trait ToWire: Sized {
    type Wire: ToApi<Api = Self> + serde::Serialize + serde::de::DeserializeOwned;

    fn to_wire(self) -> Self::Wire;
}

/// Convert from an EdenAPI Wire type to API type
pub trait ToApi: Send + Sized {
    type Api: ToWire<Wire = Self>;
    type Error: Into<WireToApiConversionError> + Send + Sync + std::error::Error;

    fn to_api(self) -> Result<Self::Api, Self::Error>;
}

impl<A: ToWire> ToWire for Vec<A> {
    type Wire = Vec<<A as ToWire>::Wire>;

    fn to_wire(self) -> Self::Wire {
        let mut out = Vec::with_capacity(self.len());
        for v in self.into_iter() {
            out.push(v.to_wire())
        }
        out
    }
}

impl<W: ToApi> ToApi for Vec<W> {
    type Api = Vec<<W as ToApi>::Api>;
    type Error = <W as ToApi>::Error;

    fn to_api(self) -> Result<Self::Api, Self::Error> {
        let mut out = Vec::with_capacity(self.len());
        for v in self.into_iter() {
            out.push(v.to_api()?)
        }
        Ok(out)
    }
}

// if needed, use macros to implement for more tuples
impl<A: ToWire, B: ToWire> ToWire for (A, B) {
    type Wire = (<A as ToWire>::Wire, <B as ToWire>::Wire);

    fn to_wire(self) -> Self::Wire {
        (self.0.to_wire(), self.1.to_wire())
    }
}

impl<A: ToApi, B: ToApi> ToApi for (A, B) {
    type Api = (<A as ToApi>::Api, <B as ToApi>::Api);
    type Error = WireToApiConversionError;

    fn to_api(self) -> Result<Self::Api, Self::Error> {
        Ok((
            self.0.to_api().map_err(|e| e.into())?,
            self.1.to_api().map_err(|e| e.into())?,
        ))
    }
}

impl<A: ToWire> ToWire for Option<A> {
    type Wire = Option<<A as ToWire>::Wire>;

    fn to_wire(self) -> Self::Wire {
        self.map(|a| a.to_wire())
    }
}

impl<W: ToApi> ToApi for Option<W> {
    type Api = Option<<W as ToApi>::Api>;
    type Error = <W as ToApi>::Error;
    fn to_api(self) -> Result<Self::Api, Self::Error> {
        self.map(|w| w.to_api()).transpose()
    }
}

// This allows using these objects as pure requests and responses
// Only use it for very simple objects which serializations don't
// incur extra costs
macro_rules! transparent_wire {
    ( $($name: ty),* ) => {
        $(
        impl ToWire for $name {
            type Wire = $name;

            fn to_wire(self) -> Self::Wire {
                self
            }
        }

        impl ToApi for $name {
            type Api = $name;
            type Error = std::convert::Infallible;

            fn to_api(self) -> Result<Self::Api, Self::Error> {
                Ok(self)
            }
        }
     )*
    }
}

transparent_wire!(bool, u8, i8, u16, i16, u32, i32, u64, i64, bytes::Bytes);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireEdenApiServerError {
    #[serde(rename = "1")]
    OpaqueError(String),

    #[serde(other, rename = "0")]
    Unknown,
}

impl ToWire for EdenApiServerErrorKind {
    type Wire = WireEdenApiServerError;

    fn to_wire(self) -> Self::Wire {
        use EdenApiServerErrorKind::*;
        match self {
            OpaqueError(s) => WireEdenApiServerError::OpaqueError(s),
        }
    }
}

impl ToApi for WireEdenApiServerError {
    type Api = EdenApiServerErrorKind;
    type Error = WireToApiConversionError;

    fn to_api(self) -> Result<Self::Api, Self::Error> {
        use WireEdenApiServerError::*;
        Ok(match self {
            Unknown => {
                return Err(WireToApiConversionError::UnrecognizedEnumVariant(
                    "WireEdenApiServerError",
                ));
            }
            OpaqueError(s) => EdenApiServerErrorKind::OpaqueError(s),
        })
    }
}

wire_hash! {
    wire => WireHgId,
    api  => HgId,
    size => 20,
}

impl fmt::Display for WireHgId {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self.to_api() {
            Ok(api) => fmt::Display::fmt(&api, fmt),
            Err(e) => match e {},
        }
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireRepoPathBuf(
    #[serde(rename = "0", default, skip_serializing_if = "is_default")] String,
);

impl ToWire for RepoPathBuf {
    type Wire = WireRepoPathBuf;

    fn to_wire(self) -> Self::Wire {
        WireRepoPathBuf(self.into_string())
    }
}

impl ToApi for WireRepoPathBuf {
    type Api = RepoPathBuf;
    type Error = RepoPathParseError;

    fn to_api(self) -> Result<Self::Api, Self::Error> {
        Ok(RepoPathBuf::from_string(self.0)?)
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireKey {
    #[serde(rename = "0", default, skip_serializing_if = "is_default")]
    path: WireRepoPathBuf,

    #[serde(rename = "1", default, skip_serializing_if = "is_default")]
    hgid: WireHgId,
}

impl ToWire for Key {
    type Wire = WireKey;

    fn to_wire(self) -> Self::Wire {
        WireKey {
            path: self.path.to_wire(),
            hgid: self.hgid.to_wire(),
        }
    }
}

impl ToApi for WireKey {
    type Api = Key;
    type Error = WireToApiConversionError;

    fn to_api(self) -> Result<Self::Api, Self::Error> {
        Ok(Key {
            path: self.path.to_api()?,
            hgid: self.hgid.to_api()?,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireParents {
    #[serde(rename = "1")]
    None,

    #[serde(rename = "2")]
    One(WireHgId),

    #[serde(rename = "3")]
    Two(WireHgId, WireHgId),

    #[serde(other, rename = "0")]
    Unknown,
}

impl ToWire for Parents {
    type Wire = WireParents;

    fn to_wire(self) -> Self::Wire {
        use Parents::*;
        match self {
            None => WireParents::None,
            One(id) => WireParents::One(id.to_wire()),
            Two(id1, id2) => WireParents::Two(id1.to_wire(), id2.to_wire()),
        }
    }
}

impl ToApi for WireParents {
    type Api = Parents;
    type Error = WireToApiConversionError;

    fn to_api(self) -> Result<Self::Api, Self::Error> {
        use WireParents::*;
        Ok(match self {
            Unknown => {
                return Err(WireToApiConversionError::UnrecognizedEnumVariant(
                    "WireParents",
                ));
            }
            None => Parents::None,
            One(id) => Parents::One(id.to_api()?),
            Two(id1, id2) => Parents::Two(id1.to_api()?, id2.to_api()?),
        })
    }
}

impl Default for WireParents {
    fn default() -> Self {
        WireParents::None
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireRevisionstoreMetadata {
    #[serde(rename = "0", default, skip_serializing_if = "is_default")]
    size: Option<u64>,

    #[serde(rename = "1", default, skip_serializing_if = "is_default")]
    flags: Option<u64>,
}

impl ToWire for RevisionstoreMetadata {
    type Wire = WireRevisionstoreMetadata;

    fn to_wire(self) -> Self::Wire {
        WireRevisionstoreMetadata {
            size: self.size,
            flags: self.flags,
        }
    }
}

impl ToApi for WireRevisionstoreMetadata {
    type Api = RevisionstoreMetadata;
    type Error = Infallible;

    fn to_api(self) -> Result<Self::Api, Self::Error> {
        Ok(RevisionstoreMetadata {
            size: self.size,
            flags: self.flags,
        })
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Ord, PartialOrd)]
#[derive(Serialize, Deserialize)]
pub struct WireDagId(u64);

impl ToWire for DagId {
    type Wire = WireDagId;

    fn to_wire(self) -> Self::Wire {
        WireDagId(self.0)
    }
}

impl ToApi for WireDagId {
    type Api = DagId;
    type Error = Infallible;

    fn to_api(self) -> Result<Self::Api, Self::Error> {
        Ok(DagId(self.0))
    }
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    v == &T::default()
}

#[cfg(any(test, feature = "for-tests"))]
impl Arbitrary for WireRepoPathBuf {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        RepoPathBuf::arbitrary(g).to_wire()
    }
}

#[cfg(any(test, feature = "for-tests"))]
impl Arbitrary for WireKey {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        Key::arbitrary(g).to_wire()
    }
}

#[cfg(any(test, feature = "for-tests"))]
impl Arbitrary for WireEdenApiServerError {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        use rand::Rng;
        let variant = g.gen_range(0, 2);
        match variant {
            0 => WireEdenApiServerError::Unknown,
            1 => WireEdenApiServerError::OpaqueError(Arbitrary::arbitrary(g)),
            _ => unreachable!(),
        }
    }
}

#[cfg(any(test, feature = "for-tests"))]
impl Arbitrary for WireParents {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        Parents::arbitrary(g).to_wire()
    }
}

#[cfg(any(test, feature = "for-tests"))]
impl Arbitrary for WireRevisionstoreMetadata {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        Self {
            size: Arbitrary::arbitrary(g),
            flags: Arbitrary::arbitrary(g),
        }
    }
}

#[cfg(any(test, feature = "for-tests"))]
impl Arbitrary for WireDagId {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        DagId::arbitrary(g).to_wire()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use quickcheck::quickcheck;

    pub fn check_serialize_roundtrip<
        T: serde::Serialize + serde::de::DeserializeOwned + Clone + PartialEq,
    >(
        original: T,
    ) -> bool {
        let serial = serde_cbor::to_vec(&original).unwrap();
        let roundtrip = serde_cbor::from_slice(&serial).unwrap();
        original == roundtrip
    }

    pub fn check_wire_roundtrip<T>(original: T) -> bool
    where
        T: ToWire + Clone + PartialEq,
        <T as ToWire>::Wire: ToApi<Api = T>,
        <<T as ToWire>::Wire as ToApi>::Error: std::fmt::Debug,
    {
        let wire = original.clone().to_wire();
        let roundtrip = wire.to_api().unwrap();
        original == roundtrip
    }

    quickcheck! {
        // Wire serialize roundtrips
        fn test_hgid_roundtrip_serialize(v: WireHgId) -> bool {
            check_serialize_roundtrip(v)
        }

        fn test_key_roundtrip_serialize(v: WireKey) -> bool {
            check_serialize_roundtrip(v)
        }

        fn test_path_roundtrip_serialize(v: WireRepoPathBuf) -> bool {
            check_serialize_roundtrip(v)
        }

        fn test_parents_roundtrip_serialize(v: WireParents) -> bool {
            check_serialize_roundtrip(v)
        }

        fn test_meta_roundtrip_serialize(v: WireRevisionstoreMetadata) -> bool {
            check_serialize_roundtrip(v)
        }

        fn test_error_roundtrip_serialize(v: WireEdenApiServerError) -> bool {
            check_serialize_roundtrip(v)
        }

        fn test_dagid_roundtrip_serialize(v: WireDagId) -> bool {
            check_serialize_roundtrip(v)
        }

        // API-Wire roundtrips
        fn test_hgid_roundtrip_wire(v: HgId) -> bool {
            check_wire_roundtrip(v)
        }

        fn test_key_roundtrip_wire(v: Key) -> bool {
            check_wire_roundtrip(v)
        }

        fn test_path_roundtrip_wire(v: RepoPathBuf) -> bool {
            check_wire_roundtrip(v)
        }

        fn test_parents_roundtrip_wire(v: Parents) -> bool {
            check_wire_roundtrip(v)
        }

        fn test_meta_roundtrip_wire(v: RevisionstoreMetadata) -> bool {
            check_wire_roundtrip(v)
        }

        fn test_dagid_roundtrip_wire(v: DagId) -> bool {
            check_wire_roundtrip(v)
        }
    }
}
