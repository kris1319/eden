/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#pragma once

#include <memory>
#include <optional>
#include <vector>

#include <folly/portability/SysTypes.h>
#include "eden/fs/model/RootId.h"
#include "eden/fs/store/ImportPriority.h"

namespace folly {
template <typename T>
class Future;
struct Unit;
} // namespace folly

namespace facebook::eden {

class Blob;
class BlobMetadata;
class Hash20;
using Hash = Hash20;
class Tree;
class ObjectFetchContext;

class IObjectStore {
 public:
  virtual ~IObjectStore() {}

  /*
   * Object access APIs.
   *
   * The given ObjectFetchContext must remain valid at least until the
   * resulting future is complete.
   */

  virtual folly::Future<std::shared_ptr<const Tree>> getRootTree(
      const RootId& rootId,
      ObjectFetchContext& context) const = 0;
  virtual folly::Future<std::shared_ptr<const Tree>> getTree(
      const Hash& id,
      ObjectFetchContext& context) const = 0;
  virtual folly::Future<std::shared_ptr<const Blob>> getBlob(
      const Hash& id,
      ObjectFetchContext& context) const = 0;

  /**
   * Prefetch all the blobs represented by the HashRange.
   *
   * The caller is responsible for making sure that the HashRange stays valid
   * for as long as the returned SemiFuture.
   */
  virtual folly::Future<folly::Unit> prefetchBlobs(
      HashRange ids,
      ObjectFetchContext& context) const = 0;
};

} // namespace facebook::eden
