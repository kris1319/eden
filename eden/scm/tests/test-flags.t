#chg-compatible
  $ setconfig experimental.allowfilepeer=True

#require execbit

  $ umask 027

  $ hg init test1
  $ cd test1
  $ touch a b
  $ hg add a b
  $ hg ci -m "added a b"

  $ cd ..
  $ hg clone test1 test3
  updating to branch default
  2 files updated, 0 files merged, 0 files removed, 0 files unresolved

  $ hg init test2
  $ cd test2
  $ hg pull ../test1
  pulling from ../test1
  requesting all changes
  adding changesets
  adding manifests
  adding file changes
  added 1 changesets with 2 changes to 2 files
  $ hg co
  2 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ chmod +x a
  $ hg ci -m "chmod +x a"

the changelog should mention file a:

  $ hg tip --template '{files}\n'
  a

  $ cd ../test1
  $ echo 123 >>a
  $ hg ci -m "a updated"

  $ hg pull ../test2
  pulling from ../test2
  searching for changes
  adding changesets
  adding manifests
  adding file changes
  added 1 changesets with 0 changes to 0 files
  $ hg heads
  commit:      7f4313b42a34
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     chmod +x a
  
  commit:      c6ecefc45368
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     a updated
  
  $ hg history
  commit:      7f4313b42a34
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     chmod +x a
  
  commit:      c6ecefc45368
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     a updated
  
  commit:      22a449e20da5
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     added a b
  

  $ hg -v merge
  resolving manifests
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  (branch merge, don't forget to commit)
  $ cat a
  123
  $ [ -x a ]

  $ cd ../test3
  $ echo 123 >>b
  $ hg ci -m "b updated"

  $ hg pull ../test2
  pulling from ../test2
  searching for changes
  adding changesets
  adding manifests
  adding file changes
  added 1 changesets with 0 changes to 0 files
  $ hg heads
  commit:      7f4313b42a34
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     chmod +x a
  
  commit:      dc57ead75f79
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     b updated
  
  $ hg history
  commit:      7f4313b42a34
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     chmod +x a
  
  commit:      dc57ead75f79
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     b updated
  
  commit:      22a449e20da5
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     added a b
  

  $ hg -v merge
  resolving manifests
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  (branch merge, don't forget to commit)

  $ ls -l ../test[123]/a > foo
  $ cut -b 1-10 < foo
  -rwxr-x---
  -rwxr-x---
  -rwxr-x---

  $ hg debugindex a
     rev    offset  length  ..... linkrev nodeid       p1           p2 (re)
       0         0       0  .....       0 b80de5d13875 000000000000 000000000000 (re)
  $ hg debugindex -R ../test2 a
     rev    offset  length  ..... linkrev nodeid       p1           p2 (re)
       0         0       0  .....       0 b80de5d13875 000000000000 000000000000 (re)
  $ hg debugindex -R ../test1 a
     rev    offset  length  ..... linkrev nodeid       p1           p2 (re)
       0         0       0  .....       0 b80de5d13875 000000000000 000000000000 (re)
       1         0       5  .....       1 7fe919cc0336 b80de5d13875 000000000000 (re)

  $ cd ..
