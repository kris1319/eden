/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#pragma once

#include "BackingStore.h"
#include "eden/fs/model/RootId.h"
#include "eden/fs/store/ObjectFetchContext.h"

namespace facebook::eden {

/*
 * A dummy BackingStore implementation, that always throws std::domain_error
 * for any ID that is looked up.
 */
class EmptyBackingStore final : public BackingStore {
 public:
  EmptyBackingStore();
  ~EmptyBackingStore() override;

  RootId parseRootId(folly::StringPiece rootId) override;
  std::string renderRootId(const RootId& rootId) override;

  folly::SemiFuture<std::unique_ptr<Tree>> getRootTree(
      const RootId& rootId,
      ObjectFetchContext& context) override;
  folly::SemiFuture<std::unique_ptr<TreeEntry>> getTreeEntryForRootId(
      const RootId& /* rootId */,
      TreeEntryType /* treeEntryType */,
      facebook::eden::PathComponentPiece /* pathComponentPiece */,
      ObjectFetchContext& /* context */) override {
    throw std::domain_error("unimplemented");
  }
  folly::SemiFuture<BackingStore::GetTreeRes> getTree(
      const Hash& id,
      ObjectFetchContext& context) override;
  folly::SemiFuture<BackingStore::GetBlobRes> getBlob(
      const Hash& id,
      ObjectFetchContext& context) override;
};

} // namespace facebook::eden
