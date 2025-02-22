/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#pragma once

#include "eden/fs/prjfs/PrjfsDispatcher.h"

namespace facebook::eden {

class EdenMount;

class PrjfsDispatcherImpl : public PrjfsDispatcher {
 public:
  explicit PrjfsDispatcherImpl(EdenMount* mount);

  folly::Future<std::vector<FileMetadata>> opendir(
      RelativePath path,
      ObjectFetchContext& context) override;

  folly::Future<std::optional<LookupResult>> lookup(
      RelativePath path,
      ObjectFetchContext& context) override;

  folly::Future<bool> access(RelativePath path, ObjectFetchContext& context)
      override;

  folly::Future<std::string> read(
      RelativePath path,
      ObjectFetchContext& context) override;

  folly::Future<folly::Unit> fileCreated(
      RelativePath relPath,
      ObjectFetchContext& context) override;

  folly::Future<folly::Unit> dirCreated(
      RelativePath relPath,
      ObjectFetchContext& context) override;

  folly::Future<folly::Unit> fileModified(
      RelativePath relPath,
      ObjectFetchContext& context) override;

  folly::Future<folly::Unit> fileRenamed(
      RelativePath oldPath,
      RelativePath newPath,
      ObjectFetchContext& context) override;

  folly::Future<folly::Unit> fileDeleted(
      RelativePath oldPath,
      ObjectFetchContext& context) override;

  folly::Future<folly::Unit> dirDeleted(
      RelativePath oldPath,
      ObjectFetchContext& context) override;

 private:
  // The EdenMount associated with this dispatcher.
  EdenMount* const mount_;

  const std::string dotEdenConfig_;
};

} // namespace facebook::eden
