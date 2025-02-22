# Copyright (c) Facebook, Inc. and its affiliates.
# Copyright (c) Mercurial Contributors.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License version 2 or any later version.

from __future__ import absolute_import

from testutil.dott import feature, sh, testtmp  # noqa: F401


sh % "setconfig experimental.allowfilepeer=True"
sh % "setconfig 'extensions.treemanifest=!'"
# Create an empty repo:

sh % "hg init a"
sh % "cd a"

# Try some commands:

sh % "hg log"
sh % "hg histgrep wah" == "[1]"
sh % "hg manifest"
sh % "hg verify" == "warning: verify does not actually check anything in this repo"

# Check the basic files created:

sh % "ls .hg" == r"""
    00changelog.i
    blackbox
    hgrc.dynamic
    reponame
    requires
    store
    treestate"""

# Should be empty:
# It's not really empty, though.

sh % "ls .hg/store" == r"""
    allheads
    metalog
    requires"""

# Poke at a clone:

sh % "cd .."
sh % "hg clone a b" == r"""
    updating to branch default
    0 files updated, 0 files merged, 0 files removed, 0 files unresolved"""
sh % "cd b"
sh % "hg verify" == "warning: verify does not actually check anything in this repo"
sh % "ls .hg" == r"""
    00changelog.i
    blackbox
    dirstate
    hgrc
    hgrc.dynamic
    reponame
    requires
    store
    treestate
    undo.branch
    undo.desc
    undo.dirstate"""

# Should be empty:
# It's not really empty, though.

sh % "ls .hg/store" == r"""
    00changelog.d
    00changelog.i
    00changelog.len
    allheads
    metalog
    requires
    undo
    undo.backupfiles
    undo.bookmarks
    undo.phaseroots"""

sh % "cd .."
