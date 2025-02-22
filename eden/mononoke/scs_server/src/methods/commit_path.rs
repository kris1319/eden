/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use bytes::Bytes;
use context::CoreContext;
use dedupmap::DedupMap;
use futures::stream::TryStreamExt;
use futures::{future, try_join};
use maplit::btreeset;
use mononoke_api::MononokePath;
use mononoke_api::{ChangesetPathHistoryOptions, ChangesetSpecifier, MononokeError, PathEntry};
use source_control as thrift;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::convert::TryFrom;

use crate::commit_id::map_commit_identities;
use crate::errors;
use crate::from_request::{check_range_and_convert, validate_timestamp};
use crate::history::collect_history;
use crate::into_response::IntoResponse;
use crate::source_control_impl::SourceControlServiceImpl;

const BLAME_TITLE_MAX_LENGTH: usize = 128;

impl SourceControlServiceImpl {
    /// Determine whether anything exists at this path.
    pub(crate) async fn commit_path_exists(
        &self,
        ctx: CoreContext,
        commit_path: thrift::CommitPathSpecifier,
        _params: thrift::CommitPathExistsParams,
    ) -> Result<thrift::CommitPathExistsResponse, errors::ServiceError> {
        let (_repo, changeset) = self.repo_changeset(ctx, &commit_path.commit).await?;
        let path = changeset.path(&commit_path.path)?;
        Ok(thrift::CommitPathExistsResponse {
            exists: path.exists().await?,
            file_exists: path.is_file().await?,
            tree_exists: path.is_tree().await?,
        })
    }

    /// Returns information about the file or directory at a path in a commit.
    pub(crate) async fn commit_path_info(
        &self,
        ctx: CoreContext,
        commit_path: thrift::CommitPathSpecifier,
        _params: thrift::CommitPathInfoParams,
    ) -> Result<thrift::CommitPathInfoResponse, errors::ServiceError> {
        let (_repo, changeset) = self.repo_changeset(ctx, &commit_path.commit).await?;
        let path = changeset.path_with_content(&commit_path.path)?;
        let response = match path.entry().await? {
            PathEntry::NotPresent => thrift::CommitPathInfoResponse {
                exists: false,
                r#type: None,
                info: None,
            },
            PathEntry::Tree(tree) => {
                let summary = tree.summary().await?;
                let tree_info = thrift::TreeInfo {
                    id: tree.id().as_ref().to_vec(),
                    simple_format_sha1: summary.simple_format_sha1.as_ref().to_vec(),
                    simple_format_sha256: summary.simple_format_sha256.as_ref().to_vec(),
                    child_files_count: summary.child_files_count as i64,
                    child_files_total_size: summary.child_files_total_size as i64,
                    child_dirs_count: summary.child_dirs_count as i64,
                    descendant_files_count: summary.descendant_files_count as i64,
                    descendant_files_total_size: summary.descendant_files_total_size as i64,
                };
                thrift::CommitPathInfoResponse {
                    exists: true,
                    r#type: Some(thrift::EntryType::TREE),
                    info: Some(thrift::EntryInfo::tree(tree_info)),
                }
            }
            PathEntry::File(file, file_type) => {
                let metadata = file.metadata().await?;
                let file_info = thrift::FileInfo {
                    id: metadata.content_id.as_ref().to_vec(),
                    file_size: metadata.total_size as i64,
                    content_sha1: metadata.sha1.as_ref().to_vec(),
                    content_sha256: metadata.sha256.as_ref().to_vec(),
                };
                thrift::CommitPathInfoResponse {
                    exists: true,
                    r#type: Some(file_type.into_response()),
                    info: Some(thrift::EntryInfo::file(file_info)),
                }
            }
        };
        Ok(response)
    }

    pub(crate) async fn commit_multiple_path_info(
        &self,
        ctx: CoreContext,
        commit: thrift::CommitSpecifier,
        params: thrift::CommitMultiplePathInfoParams,
    ) -> Result<thrift::CommitMultiplePathInfoResponse, errors::ServiceError> {
        let (_repo, changeset) = self.repo_changeset(ctx, &commit).await?;
        let mut paths = vec![];
        for path in params.paths {
            let strpath = path.as_str();
            let mpath = MononokePath::try_from(strpath)?;
            paths.push(mpath);
        }

        let result = changeset
            .paths_with_content(paths.into_iter())
            .await?
            .map_ok(|context| async move {
                let context_path = context.path().to_string();

                match context.entry().await? {
                    PathEntry::NotPresent => {
                        let not_present_elem = thrift::CommitPathInfoResponse {
                            exists: false,
                            r#type: None,
                            info: None,
                        };
                        return Result::<_, errors::ServiceError>::Ok((
                            context_path,
                            not_present_elem,
                        ));
                    }
                    PathEntry::Tree(tree) => {
                        let summary = tree.summary().await?;
                        let tree_elem = thrift::CommitPathInfoResponse {
                            exists: true,
                            r#type: Some(thrift::EntryType::TREE),
                            info: Some(thrift::EntryInfo::tree(
                                (*tree.id(), summary).into_response(),
                            )),
                        };
                        return Result::<_, errors::ServiceError>::Ok((context_path, tree_elem));
                    }
                    PathEntry::File(file, file_type) => {
                        let metadata = file.metadata().await?;
                        let file_elem = thrift::CommitPathInfoResponse {
                            exists: true,
                            r#type: Some(file_type.into_response()),
                            info: Some(thrift::EntryInfo::file(metadata.into_response())),
                        };
                        return Result::<_, errors::ServiceError>::Ok((context_path, file_elem));
                    }
                };
            })
            .map_err(errors::ServiceError::from)
            .try_buffer_unordered(100)
            .try_collect::<BTreeMap<_, _>>()
            .await?;

        Ok(thrift::CommitMultiplePathInfoResponse { path_info: result })
    }

    pub(crate) async fn commit_path_blame(
        &self,
        ctx: CoreContext,
        commit_path: thrift::CommitPathSpecifier,
        params: thrift::CommitPathBlameParams,
    ) -> Result<thrift::CommitPathBlameResponse, errors::ServiceError> {
        match params.format {
            thrift::BlameFormat::COMPACT => {
                self.commit_path_blame_compact(ctx, commit_path, params)
                    .await
            }
            other_format => Err(errors::invalid_request(format!(
                "unsupported blame format {}",
                other_format
            ))
            .into()),
        }
    }

    async fn commit_path_blame_compact(
        &self,
        ctx: CoreContext,
        commit_path: thrift::CommitPathSpecifier,
        params: thrift::CommitPathBlameParams,
    ) -> Result<thrift::CommitPathBlameResponse, errors::ServiceError> {
        let (repo, changeset) = self.repo_changeset(ctx, &commit_path.commit).await?;
        let path = changeset.path_with_history(&commit_path.path)?;

        let options = params.format_options.unwrap_or_else(|| {
            btreeset! { thrift::BlameFormatOption::INCLUDE_CONTENTS }
        });
        let option_include_contents =
            options.contains(&thrift::BlameFormatOption::INCLUDE_CONTENTS);
        let option_include_title = options.contains(&thrift::BlameFormatOption::INCLUDE_TITLE);
        let option_include_message = options.contains(&thrift::BlameFormatOption::INCLUDE_MESSAGE);

        let mut commit_ids = Vec::new();
        let mut commit_id_indexes = HashMap::new();
        let mut paths = DedupMap::new();
        let mut authors = DedupMap::new();
        let mut dates = DedupMap::new();
        let mut titles = DedupMap::new();
        let mut messages = DedupMap::new();

        // Fetch the blame, and optionally its associated content.
        let (blame, content) = if option_include_contents {
            path.blame_with_content().await?
        } else {
            (path.blame().await?, Bytes::new())
        };

        // Map all the changeset IDs into the requested identity schemes.  Keep a mapping of
        // which bonsai changeset ID corresponds to which mapped commit ID index, so we can look
        // them up later.
        let csids: Vec<_> = blame
            .changeset_ids()
            .map_err(|e| MononokeError::InvalidRequest(e.to_string()))?;
        for (id, mapped_ids) in
            map_commit_identities(&repo, csids.clone(), &params.identity_schemes)
                .await?
                .into_iter()
        {
            let index = commit_ids.len();
            commit_ids.push(mapped_ids);
            commit_id_indexes.insert(id, index);
        }

        // Collect author and date fields from the commit info.
        let info: HashMap<_, _> = future::try_join_all(csids.into_iter().map(move |csid| {
            let repo = repo.clone();
            async move {
                let changeset = repo
                    .changeset(ChangesetSpecifier::Bonsai(csid))
                    .await?
                    .ok_or_else(|| {
                        MononokeError::InvalidRequest(format!("failed to resolve commit: {}", csid))
                    })?;
                let (date, author, message) = try_join!(
                    changeset.author_date(),
                    changeset.author(),
                    changeset.message(),
                )?;
                let title: String = message
                    .chars()
                    .take(BLAME_TITLE_MAX_LENGTH)
                    .take_while(|ch| *ch != '\n')
                    .collect();

                Ok::<_, MononokeError>((csid, (author, date, message, title)))
            }
        }))
        .await?
        .into_iter()
        .collect();

        let mut content_iter = content.as_ref().split(|c| *c == b'\n');

        let lines = blame
            .lines()
            .map_err(|e| MononokeError::InvalidRequest(e.to_string()))?
            .enumerate()
            .map(|(line, blame_line)| -> Result<_, thrift::RequestError> {
                let commit_id_index =
                    commit_id_indexes
                        .get(&blame_line.changeset_id)
                        .ok_or_else(|| {
                            errors::commit_not_found(format!(
                                "failed to resolve commit: {}",
                                blame_line.changeset_id
                            ))
                        })?;
                let (author, date, message, title) =
                    info.get(&blame_line.changeset_id).ok_or_else(|| {
                        errors::commit_not_found(format!(
                            "failed to resolve commit: {}",
                            blame_line.changeset_id
                        ))
                    })?;
                let mut blame_line = thrift::BlameCompactLine {
                    line: (line + 1) as i32,
                    contents: None,
                    commit_id_index: *commit_id_index as i32,
                    path_index: paths.insert(&blame_line.path.to_string()) as i32,
                    author_index: authors.insert(author) as i32,
                    date_index: dates.insert(Cow::Borrowed(date)) as i32,
                    origin_line: (blame_line.origin_offset + 1) as i32,
                    title_index: None,
                    message_index: None,
                };
                if option_include_contents {
                    if let Some(content_line) = content_iter.next() {
                        blame_line.contents =
                            Some(String::from_utf8_lossy(content_line).into_owned());
                    }
                }
                if option_include_title {
                    blame_line.title_index = Some(titles.insert(title) as i32);
                }
                if option_include_message {
                    blame_line.message_index = Some(messages.insert(message) as i32);
                }
                Ok(blame_line)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let paths = paths.into_items();
        let authors = authors.into_items();
        let titles = Some(titles.into_items()).filter(|titles| !titles.is_empty());
        let messages = Some(messages.into_items()).filter(|messages| !messages.is_empty());
        let dates = dates
            .into_items()
            .into_iter()
            .map(|date| thrift::DateTime {
                timestamp: date.timestamp(),
                tz: date.offset().local_minus_utc(),
            })
            .collect();
        let blame = thrift::BlameCompact {
            lines,
            commit_ids,
            paths,
            authors,
            dates,
            titles,
            messages,
        };

        Ok(thrift::CommitPathBlameResponse {
            blame: thrift::Blame::blame_compact(blame),
        })
    }

    pub(crate) async fn commit_path_history(
        &self,
        ctx: CoreContext,
        commit_path: thrift::CommitPathSpecifier,
        params: thrift::CommitPathHistoryParams,
    ) -> Result<thrift::CommitPathHistoryResponse, errors::ServiceError> {
        let (repo, changeset) = self.repo_changeset(ctx, &commit_path.commit).await?;
        let path = changeset.path_with_history(&commit_path.path)?;
        let (descendants_of, exclude_changeset_and_ancestors) = try_join!(
            async {
                if let Some(descendants_of) = &params.descendants_of {
                    Ok::<_, errors::ServiceError>(Some(
                        self.changeset_id(&repo, &descendants_of).await?,
                    ))
                } else {
                    Ok(None)
                }
            },
            async {
                if let Some(exclude_changeset_and_ancestors) =
                    &params.exclude_changeset_and_ancestors
                {
                    Ok::<_, errors::ServiceError>(Some(
                        self.changeset_id(&repo, &exclude_changeset_and_ancestors)
                            .await?,
                    ))
                } else {
                    Ok(None)
                }
            }
        )?;

        let limit: usize = check_range_and_convert("limit", params.limit, 0..)?;
        let skip: usize = check_range_and_convert("skip", params.skip, 0..)?;

        // Time filter equal to zero might be mistaken by users for an unset, like None.
        // We will consider negative timestamps as invalid and zeros as unset.
        let after_timestamp = validate_timestamp(params.after_timestamp, "after_timestamp")?;
        let before_timestamp = validate_timestamp(params.before_timestamp, "before_timestamp")?;

        if let (Some(ats), Some(bts)) = (after_timestamp, before_timestamp) {
            if bts < ats {
                return Err(errors::invalid_request(format!(
                    "after_timestamp ({}) cannot be greater than before_timestamp ({})",
                    ats, bts,
                ))
                .into());
            }
        }

        if skip > 0 && (after_timestamp.is_some() || before_timestamp.is_some()) {
            return Err(errors::invalid_request(
                "Time filters cannot be applied if skip is not 0".to_string(),
            )
            .into());
        }

        let history_stream = path
            .history(ChangesetPathHistoryOptions {
                until_timestamp: after_timestamp.clone(),
                descendants_of,
                exclude_changeset_and_ancestors,
                follow_history_across_deletions: params.follow_history_across_deletions,
            })
            .await?;
        let history = collect_history(
            history_stream,
            skip,
            limit,
            before_timestamp,
            after_timestamp,
            params.format,
            &params.identity_schemes,
        )
        .await?;

        Ok(thrift::CommitPathHistoryResponse { history })
    }
}
