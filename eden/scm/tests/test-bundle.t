#chg-compatible
  $ setconfig experimental.allowfilepeer=True

  $ disable treemanifest
  $ setconfig format.usegeneraldelta=yes

Setting up test

  $ hg init test
  $ cd test
  $ echo 0 > afile
  $ hg add afile
  $ hg commit -m "0.0"
  $ echo 1 >> afile
  $ hg commit -m "0.1"
  $ echo 2 >> afile
  $ hg commit -m "0.2"
  $ echo 3 >> afile
  $ hg commit -m "0.3"
  $ hg update -C 'desc(0.0)'
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ echo 1 >> afile
  $ hg commit -m "1.1"
  $ echo 2 >> afile
  $ hg commit -m "1.2"
  $ echo "a line" > fred
  $ echo 3 >> afile
  $ hg add fred
  $ hg commit -m "1.3"
  $ hg mv afile adifferentfile
  $ hg commit -m "1.3m"
  $ hg update -C 'desc(0.3)'
  1 files updated, 0 files merged, 2 files removed, 0 files unresolved
  $ hg mv afile anotherfile
  $ hg commit -m "0.3m"
  $ hg verify
  warning: verify does not actually check anything in this repo
  $ cd ..
  $ hg init empty

Bundle --all

  $ hg -R test bundle --all all.hg
  9 changesets found

Bundle test to full.hg

  $ hg -R test bundle full.hg empty
  searching for changes
  9 changesets found

Unbundle full.hg in test

  $ hg -R test unbundle full.hg
  adding changesets
  adding manifests
  adding file changes
  added 0 changesets with 0 changes to 4 files

Verify empty

  $ hg -R empty heads
  [1]
  $ hg -R empty verify
  warning: verify does not actually check anything in this repo

Pull full.hg into test (using --cwd)

  $ hg --cwd test pull ../full.hg
  pulling from ../full.hg
  searching for changes
  no changes found

Verify that there are no leaked temporary files after pull (issue2797)

  $ ls test/.hg | grep .hg10un
  [1]

Pull full.hg into empty (using --cwd)

  $ hg --cwd empty pull ../full.hg
  pulling from ../full.hg
  requesting all changes
  adding changesets
  adding manifests
  adding file changes
  added 9 changesets with 7 changes to 4 files

Rollback empty

  $ hg -R empty debugstrip 'desc(0.0)' --no-backup

Pull full.hg into empty again (using --cwd)

  $ hg --cwd empty pull ../full.hg
  pulling from ../full.hg
  requesting all changes
  adding changesets
  adding manifests
  adding file changes
  added 9 changesets with 0 changes to 4 files

Pull full.hg into test (using -R)

  $ hg -R test pull full.hg
  pulling from full.hg
  searching for changes
  no changes found

Pull full.hg into empty (using -R)

  $ hg -R empty pull full.hg
  pulling from full.hg
  searching for changes
  no changes found

Rollback empty

  $ hg -R empty debugstrip 'desc(0.0)' --no-backup

Pull full.hg into empty again (using -R)

  $ hg -R empty pull full.hg
  pulling from full.hg
  requesting all changes
  adding changesets
  adding manifests
  adding file changes
  added 9 changesets with 0 changes to 4 files

Log -R full.hg in fresh empty

  $ rm -r empty
  $ hg init empty
  $ cd empty
  $ hg -R ../full.hg log
  commit:      aa35859c02ea
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     0.3m
  
  commit:      a6a34bfa0076
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     1.3m
  
  commit:      7373c1169842
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     1.3
  
  commit:      1bb50a9436a7
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     1.2
  
  commit:      095197eb4973
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     1.1
  
  commit:      eebf5a27f8ca
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     0.3
  
  commit:      e38ba6f5b7e0
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     0.2
  
  commit:      34c2bf6b0626
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     0.1
  
  commit:      f9ee2f85a263
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     0.0
  
Make sure bundlerepo doesn't leak tempfiles (issue2491)

  $ ls .hg
  00changelog.i
  blackbox
  hgrc.dynamic
  reponame
  requires
  store
  treestate

Pull ../full.hg into empty (with hook)

  $ cat >> .hg/hgrc <<EOF
  > [hooks]
  > changegroup = sh -c "printenv.py changegroup"
  > EOF

doesn't work (yet ?)

hg -R ../full.hg verify

  $ hg pull bundle://../full.hg
  pulling from bundle:../full.hg
  requesting all changes
  adding changesets
  adding manifests
  adding file changes
  added 9 changesets with 7 changes to 4 files
  changegroup hook: HG_HOOKNAME=changegroup HG_HOOKTYPE=changegroup HG_NODE=f9ee2f85a263049e9ae6d37a0e67e96194ffb735 HG_NODE_LAST=aa35859c02ea8bd48da5da68cd2740ac71afcbaf HG_SOURCE=pull HG_TXNID=TXN:$ID$ HG_URL=bundle*../full.hg (glob)

Rollback empty

  $ hg debugstrip 'desc(0.0)' --no-backup
  $ cd ..

Log -R bundle:empty+full.hg (broken with Rust code path)

  $ hg -R bundle:empty+full.hg log --template="{node} "; echo ""
  abort: repository bundle:empty+full.hg not found!
  

Pull full.hg into empty again (using -R; with hook)

  $ hg -R empty pull full.hg
  pulling from full.hg
  requesting all changes
  adding changesets
  adding manifests
  adding file changes
  added 9 changesets with 0 changes to 4 files
  changegroup hook: HG_HOOKNAME=changegroup HG_HOOKTYPE=changegroup HG_NODE=f9ee2f85a263049e9ae6d37a0e67e96194ffb735 HG_NODE_LAST=aa35859c02ea8bd48da5da68cd2740ac71afcbaf HG_SOURCE=pull HG_TXNID=TXN:$ID$ HG_URL=bundle:empty+full.hg

Cannot produce streaming clone bundles with "hg bundle"

  $ hg -R test bundle -t packed1 packed.hg
  abort: packed bundles cannot be produced by "hg bundle"
  (use 'hg debugcreatestreamclonebundle')
  [255]

packed1 is produced properly

  $ hg -R test debugcreatestreamclonebundle packed.hg
  writing * bytes for 7 files (glob)
  bundle requirements: generaldelta, lz4revlog, revlogv1

#if common-zlib
  $ f -B 64 --size --sha1 --hexdump packed.hg
  packed.hg: size=2799, sha1=870ec1de2df4b7f71501812b244fd53fc5eb25f1
  0000: 48 47 53 31 55 4e 00 00 00 00 00 00 00 07 00 00 |HGS1UN..........|
  0010: 00 00 00 00 0a 31 00 20 67 65 6e 65 72 61 6c 64 |.....1. generald|
  0020: 65 6c 74 61 2c 6c 7a 34 72 65 76 6c 6f 67 2c 72 |elta,lz4revlog,r|
  0030: 65 76 6c 6f 67 76 31 00 30 30 6d 61 6e 69 66 65 |evlogv1.00manife|
#endif

  $ hg debugbundle --spec packed.hg
  none-packed1;requirements%3Dgeneraldelta%2Clz4revlog%2Crevlogv1

generaldelta requirement is not listed in stream clone bundles unless used

  $ hg --config format.usegeneraldelta=false init testnongd
  $ cd testnongd
  $ touch foo
  $ hg -q commit -A -m initial
  $ cd ..
  $ hg -R testnongd debugcreatestreamclonebundle packednongd.hg
  writing 301 bytes for 4 files
  bundle requirements: lz4revlog, revlogv1

  $ f -B 64 --size --sha1 --hexdump packednongd.hg
  packednongd.hg: size=409, sha1=344c366796aba47616a3e9a56836e8789bb2af26
  0000: 48 47 53 31 55 4e 00 00 00 00 00 00 00 04 00 00 |HGS1UN..........|
  0010: 00 00 00 00 01 2d 00 13 6c 7a 34 72 65 76 6c 6f |.....-..lz4revlo|
  0020: 67 2c 72 65 76 6c 6f 67 76 31 00 30 30 6d 61 6e |g,revlogv1.00man|
  0030: 69 66 65 73 74 2e 69 00 31 31 30 0a 00 01 00 01 |ifest.i.110.....|

  $ hg debugbundle --spec packednongd.hg
  none-packed1;requirements%3Dlz4revlog%2Crevlogv1

Unpacking packed1 bundles with "hg unbundle" isn't allowed

  $ hg init packed
  $ hg -R packed unbundle packed.hg
  abort: packed bundles cannot be applied with "hg unbundle"
  (use "hg debugapplystreamclonebundle")
  [255]

packed1 can be consumed from debug command

(this also confirms that streamclone-ed changes are visible via
@filecache properties to in-process procedures before closing
transaction)

  $ cat > $TESTTMP/showtip.py <<EOF
  > from __future__ import absolute_import
  > 
  > def showtip(ui, repo, hooktype, **kwargs):
  >     ui.warn('%s: %s\n' % (hooktype, repo['tip'].hex()[:12]))
  > 
  > def reposetup(ui, repo):
  >     # this confirms (and ensures) that (empty) 00changelog.i
  >     # before streamclone is already cached as repo.changelog
  >     ui.setconfig('hooks', 'pretxnopen.showtip', showtip)
  > 
  >     # this confirms that streamclone-ed changes are visible to
  >     # in-process procedures before closing transaction
  >     ui.setconfig('hooks', 'pretxnclose.showtip', showtip)
  > 
  >     # this confirms that streamclone-ed changes are still visible
  >     # after closing transaction
  >     ui.setconfig('hooks', 'txnclose.showtip', showtip)
  > EOF
  $ cat >> $HGRCPATH <<EOF
  > [extensions]
  > showtip = $TESTTMP/showtip.py
  > EOF

  $ hg -R packed debugapplystreamclonebundle packed.hg
  7 files to transfer, * of data (glob)
  pretxnopen: 000000000000
  pretxnclose: aa35859c02ea
  transferred 2.55 KB in 0.0 seconds (2.49 MB/sec)
  txnclose: aa35859c02ea

(for safety, confirm visibility of streamclone-ed changes by another
process, too)

  $ hg -R packed tip -T "{node|short}\n"
  aa35859c02ea

  $ cat >> $HGRCPATH <<EOF
  > [extensions]
  > showtip = !
  > EOF

Does not work on non-empty repo

  $ hg -R packed debugapplystreamclonebundle packed.hg
  abort: cannot apply stream clone bundle on non-empty repo
  [255]

Create partial clones

  $ rm -r empty
  $ hg init empty
  $ hg clone -r 3 test partial
  adding changesets
  adding manifests
  adding file changes
  added 4 changesets with 4 changes to 1 files
  updating to branch default
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ hg clone partial partial2
  updating to branch default
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ cd partial

Log -R full.hg in partial

  $ hg -R ../full.hg log -T phases
  commit:      aa35859c02ea
  phase:       draft
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     0.3m
  
  commit:      a6a34bfa0076
  phase:       draft
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     1.3m
  
  commit:      7373c1169842
  phase:       draft
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     1.3
  
  commit:      1bb50a9436a7
  phase:       draft
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     1.2
  
  commit:      095197eb4973
  phase:       draft
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     1.1
  
  commit:      eebf5a27f8ca
  phase:       draft
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     0.3
  
  commit:      e38ba6f5b7e0
  phase:       draft
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     0.2
  
  commit:      34c2bf6b0626
  phase:       draft
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     0.1
  
  commit:      f9ee2f85a263
  phase:       draft
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     0.0
  

Incoming full.hg in partial

  $ hg incoming bundle://../full.hg
  comparing with bundle:../full.hg
  searching for changes
  commit:      095197eb4973
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     1.1
  
  commit:      1bb50a9436a7
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     1.2
  
  commit:      7373c1169842
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     1.3
  
  commit:      a6a34bfa0076
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     1.3m
  
  commit:      aa35859c02ea
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     0.3m
  

Outgoing -R full.hg vs partial2 in partial

  $ hg -R ../full.hg outgoing ../partial2
  comparing with ../partial2
  searching for changes
  commit:      095197eb4973
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     1.1
  
  commit:      1bb50a9436a7
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     1.2
  
  commit:      7373c1169842
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     1.3
  
  commit:      a6a34bfa0076
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     1.3m
  
  commit:      aa35859c02ea
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     0.3m
  

Outgoing -R does-not-exist.hg vs partial2 in partial

  $ hg -R ../does-not-exist.hg outgoing ../partial2
  abort: *../does-not-exist.hg* (glob)
  [255]
  $ cd ..

hide outer repo
  $ hg init

Direct clone from bundle (all-history)

  $ hg clone full.hg full-clone
  requesting all changes
  adding changesets
  adding manifests
  adding file changes
  added 9 changesets with 7 changes to 4 files
  updating to branch default
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ hg -R full-clone heads
  commit:      aa35859c02ea
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     0.3m
  
  commit:      a6a34bfa0076
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     1.3m
  
  $ rm -r full-clone

When cloning from a non-copiable repository into '', do not
recurse infinitely (issue2528)

  $ hg clone full.hg ''
  abort: empty destination path is not valid
  [255]

test for https://bz.mercurial-scm.org/216

Unbundle incremental bundles into fresh empty in one go

  $ rm -r empty
  $ hg init empty
  $ hg -R test bundle --base null -r 'desc(0.0)' ../0.hg
  1 changesets found
  $ hg -R test bundle --base 'desc(0.0)'    -r 'desc(0.1)' ../1.hg
  1 changesets found
  $ hg -R empty unbundle -u ../0.hg ../1.hg
  adding changesets
  adding manifests
  adding file changes
  added 1 changesets with 1 changes to 1 files
  adding changesets
  adding manifests
  adding file changes
  added 1 changesets with 1 changes to 1 files
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved

View full contents of the bundle
  $ hg -R test bundle --base null -r eebf5a27f8ca9b92ade529321141c1561cc4a9c2  ../partial.hg
  4 changesets found
  $ cd test
  $ hg -R ../../partial.hg log -r "bundle()"
  commit:      f9ee2f85a263
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     0.0
  
  commit:      34c2bf6b0626
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     0.1
  
  commit:      e38ba6f5b7e0
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     0.2
  
  commit:      eebf5a27f8ca
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     0.3
  
  $ cd ..

test for 540d1059c802

test for 540d1059c802

  $ hg init orig
  $ cd orig
  $ echo foo > foo
  $ hg add foo
  $ hg ci -m 'add foo'

  $ hg clone . ../copy
  updating to branch default
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved

  $ cd ../copy
  $ echo >> foo
  $ hg ci -m 'change foo'
  $ hg bundle ../bundle.hg ../orig
  searching for changes
  1 changesets found

  $ cd ../orig
  $ hg incoming ../bundle.hg
  comparing with ../bundle.hg
  searching for changes
  commit:      ed1b79f46b9a
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     change foo
  
  $ cd ..

test bundle with # in the filename (issue2154):

  $ cp bundle.hg 'test#bundle.hg'
  $ cd orig
  $ hg incoming '../test#bundle.hg'
  comparing with ../test
  abort: unknown revision 'bundle.hg'!
  [255]

note that percent encoding is not handled:

  $ hg incoming ../test%23bundle.hg
  abort: repository ../test%23bundle.hg not found!
  [255]
  $ cd ..

test for https://bz.mercurial-scm.org/1144

test that verify bundle does not traceback

partial history bundle, fails w/ unknown parent

  $ hg -R bundle.hg verify
  abort: 00changelog.i@bbd179dfa0a7: unknown parent!
  [255]

full history bundle, refuses to verify non-local repo

  $ hg -R all.hg verify
  warning: verify does not actually check anything in this repo

but, regular verify must continue to work

  $ hg -R orig verify
  warning: verify does not actually check anything in this repo

diff against bundle

  $ hg init b
  $ cd b
  $ hg -R ../all.hg diff -r tip
  diff -r aa35859c02ea anotherfile
  --- a/anotherfile	Thu Jan 01 00:00:00 1970 +0000
  +++ /dev/null	Thu Jan 01 00:00:00 1970 +0000
  @@ -1,4 +0,0 @@
  -0
  -1
  -2
  -3
  $ cd ..

bundle single branch

  $ hg init branchy
  $ cd branchy
  $ echo a >a
  $ echo x >x
  $ hg ci -Ama
  adding a
  adding x
  $ echo c >c
  $ echo xx >x
  $ hg ci -Amc
  adding c
  $ echo c1 >c1
  $ hg ci -Amc1
  adding c1
  $ hg up 'desc(a)'
  1 files updated, 0 files merged, 2 files removed, 0 files unresolved
  $ echo b >b
  $ hg ci -Amb
  adding b
  $ echo b1 >b1
  $ echo xx >x
  $ hg ci -Amb1
  adding b1
  $ hg clone -q -r2 . part

== bundling via incoming

  $ hg in -R part --bundle incoming.hg --template "{node}\n" .
  comparing with .
  searching for changes
  1a38c1b849e8b70c756d2d80b0b9a3ac0b7ea11a
  057f4db07f61970e1c11e83be79e9d08adc4dc31

== bundling

  $ hg bundle bundle.hg part --debug --config progress.debug=true
  query 1; heads
  searching for changes
  local heads: 2; remote heads: 1 (explicit: 0); initial common: 1
  all remote heads known locally
  2 changesets found
  list of changesets:
  1a38c1b849e8b70c756d2d80b0b9a3ac0b7ea11a
  057f4db07f61970e1c11e83be79e9d08adc4dc31
  bundle2-output-bundle: "HG20", (1 params) 1 parts total
  bundle2-output-part: "changegroup" (params: 1 mandatory 1 advisory) streamed payload
  progress: bundling: 1/2 changesets (50.00%)
  progress: bundling: 2/2 changesets (100.00%)
  progress: bundling (end)
  progress: bundling: 1/2 manifests (50.00%)
  progress: bundling: 2/2 manifests (100.00%)
  progress: bundling (end)
  progress: bundling: b 1/3 files (33.33%)
  progress: bundling: b1 2/3 files (66.67%)
  progress: bundling: x 3/3 files (100.00%)
  progress: bundling (end)

== Test for issue3441

  $ hg clone -q -r0 . part2
  $ hg -q -R part2 pull bundle.hg
  $ hg -R part2 verify
  warning: verify does not actually check anything in this repo

== Test bundling no commits

  $ hg bundle -r 'public()' no-output.hg
  abort: no commits to bundle
  [255]

  $ cd ..

When user merges to the revision existing only in the bundle,
it should show warning that second parent of the working
directory does not exist

  $ hg init update2bundled
  $ cd update2bundled
  $ cat <<EOF >> .hg/hgrc
  > [extensions]
  > strip =
  > EOF
  $ echo "aaa" >> a
  $ hg commit -A -m 0
  adding a
  $ echo "bbb" >> b
  $ hg commit -A -m 1
  adding b
  $ echo "ccc" >> c
  $ hg commit -A -m 2
  adding c
  $ hg update -r 'desc(1)'
  0 files updated, 0 files merged, 1 files removed, 0 files unresolved
  $ echo "ddd" >> d
  $ hg commit -A -m 3
  adding d
  $ hg update -r 'desc(2)'
  1 files updated, 0 files merged, 1 files removed, 0 files unresolved
  $ hg log -G
  o  commit:      8bd3e1f196af
  │  user:        test
  │  date:        Thu Jan 01 00:00:00 1970 +0000
  │  summary:     3
  │
  │ @  commit:      4652c276ac4f
  ├─╯  user:        test
  │    date:        Thu Jan 01 00:00:00 1970 +0000
  │    summary:     2
  │
  o  commit:      a01eca7af26d
  │  user:        test
  │  date:        Thu Jan 01 00:00:00 1970 +0000
  │  summary:     1
  │
  o  commit:      4fe08cd4693e
     user:        test
     date:        Thu Jan 01 00:00:00 1970 +0000
     summary:     0
  
  $ hg bundle --base 'desc(1)' -r 'desc(3)' ../update2bundled.hg
  1 changesets found
  $ hg debugstrip -r 'desc(3)'
  $ hg merge -R ../update2bundled.hg -r 'desc(3)'
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  (branch merge, don't forget to commit)

When user updates to the revision existing only in the bundle,
it should show warning

  $ hg update -R ../update2bundled.hg --clean -r 'desc(3)'
  1 files updated, 0 files merged, 1 files removed, 0 files unresolved

When user updates to the revision existing in the local repository
the warning shouldn't be emitted

  $ hg update -R ../update2bundled.hg -r 'desc(0)'
  0 files updated, 0 files merged, 2 files removed, 0 files unresolved
