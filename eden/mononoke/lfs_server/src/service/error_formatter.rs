/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use anyhow::{Context, Error};
use gotham::state::{request_id, State};
use gotham_ext::error::ErrorFormatter;
use lfs_protocol::{git_lfs_mime, ResponseError};
use mime::Mime;

pub struct LfsErrorFormatter;

impl ErrorFormatter for LfsErrorFormatter {
    type Body = Vec<u8>;

    fn format(&self, error: &Error, state: &State) -> Result<(Self::Body, Mime), Error> {
        let message = format!("{:#}", error);

        let res = ResponseError {
            message,
            documentation_url: None,
            request_id: Some(request_id(&state).to_string()),
        };

        let body = serde_json::to_vec(&res).context("Failed to serialize error")?;

        Ok((body, git_lfs_mime()))
    }
}
