/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#pragma once

#include <memory>
#include "eden/fs/inodes/InodePtrFwd.h"
#include "eden/fs/utils/PathFuncs.h"

namespace folly {
template <typename T>
class Future;
struct Unit;
} // namespace folly

namespace facebook {
namespace eden {

class DiffContext;
class GitIgnoreStack;
class Hash20;
using Hash = Hash20;
class ObjectStore;
class TreeEntry;
class TreeInode;
class DiffCallback;

/**
 * A helper class for use in TreeInode::diff()
 *
 * While diff() holds the contents_ lock it computes a set of child entries
 * that need to be examined later once it releases the contents_ lock.
 * DeferredDiffEntry is used to store the data about which children need to be
 * examined.  The DeferredDiffEntry subclasses contain the logic for how to
 * then perform the diff on the child entry.
 */
class DeferredDiffEntry {
 public:
  explicit DeferredDiffEntry(DiffContext* context, RelativePath&& path)
      : context_{context}, path_{std::move(path)} {}
  virtual ~DeferredDiffEntry() {}

  const RelativePath& getPath() const {
    return path_;
  }

  FOLLY_NODISCARD virtual folly::Future<folly::Unit> run() = 0;

  static std::unique_ptr<DeferredDiffEntry> createUntrackedEntry(
      DiffContext* context,
      RelativePath path,
      InodePtr inode,
      const GitIgnoreStack* ignore,
      bool isIgnored);

  /*
   * This is named differently from the createUntrackedEntry() function above
   * just to avoid ambiguous overload calls--folly::Future<X> can unfortunately
   * be implicitly constructed from X.  We could help the compiler avoid the
   * ambiguity by making the Future<InodePtr> version of createUntrackedEntry()
   * be a template method.  However, just using a separate name is easier for
   * now.
   */
  static std::unique_ptr<DeferredDiffEntry> createUntrackedEntryFromInodeFuture(
      DiffContext* context,
      RelativePath path,
      folly::Future<InodePtr>&& inodeFuture,
      const GitIgnoreStack* ignore,
      bool isIgnored);

  static std::unique_ptr<DeferredDiffEntry> createRemovedEntry(
      DiffContext* context,
      RelativePath path,
      const TreeEntry& scmEntry);

  static std::unique_ptr<DeferredDiffEntry> createModifiedEntry(
      DiffContext* context,
      RelativePath path,
      const TreeEntry& scmEntry,
      InodePtr inode,
      const GitIgnoreStack* ignore,
      bool isIgnored);

  static std::unique_ptr<DeferredDiffEntry> createModifiedEntryFromInodeFuture(
      DiffContext* context,
      RelativePath path,
      const TreeEntry& scmEntry,
      folly::Future<InodePtr>&& inodeFuture,
      const GitIgnoreStack* ignore,
      bool isIgnored);

  static std::unique_ptr<DeferredDiffEntry> createModifiedEntry(
      DiffContext* context,
      RelativePath path,
      const TreeEntry& scmEntry,
      Hash currentBlobHash);

  static std::unique_ptr<DeferredDiffEntry> createModifiedScmEntry(
      DiffContext* context,
      RelativePath path,
      Hash scmHash,
      Hash wdHash,
      const GitIgnoreStack* ignore,
      bool isIgnored);

  static std::unique_ptr<DeferredDiffEntry> createAddedScmEntry(
      DiffContext* context,
      RelativePath path,
      Hash wdHash,
      const GitIgnoreStack* ignore,
      bool isIgnored);

  static std::unique_ptr<DeferredDiffEntry>
  createRemovedScmEntry(DiffContext* context, RelativePath path, Hash scmHash);

 protected:
  DiffContext* const context_;
  RelativePath const path_;
};
} // namespace eden
} // namespace facebook
