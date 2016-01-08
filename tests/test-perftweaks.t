  $ extpath=`dirname $TESTDIR`
  $ cp $extpath/perftweaks.py $TESTTMP # use $TESTTMP substitution in message
  $ cat >> $HGRCPATH << EOF
  > [extensions]
  > perftweaks=$TESTTMP/perftweaks.py
  > EOF

Test disabling the tag cache
  $ hg init tagcache
  $ cd tagcache
  $ cat >> .hg/hgrc <<EOF
  > [extensions]
  > blackbox=
  > EOF
  $ touch a && hg add -q a
  $ hg commit -qm "Foo"
  $ hg tag foo

  $ rm -rf .hg/cache .hg/blackbox.log
  $ hg tags
  tip                                1:2cc13e58bcd8
  foo                                0:be5a2292aa62
  $ hg blackbox | grep tag
  *> tags (glob)
  *> writing * bytes to cache/hgtagsfnodes1 (glob)
  *> writing .hg/cache/tags2-visible with 1 tags (glob)
  *> tags exited 0 after * seconds (glob)

  $ rm -rf .hg/cache .hg/blackbox.log
  $ hg tags --config perftweaks.disabletags=True
  tip                                1:2cc13e58bcd8
  $ hg blackbox | grep tag
  *> tags (glob)
  *> tags --config perftweaks.disabletags=True exited 0 after * seconds (glob)

  $ cd ..

Test disabling the case conflict check (only fails on case sensitive systems)
  $ hg init casecheck
  $ cd casecheck
  $ cat >> .hg/hgrc <<EOF
  > [perftweaks]
  > disablecasecheck=True
  > EOF
  $ touch a
  $ hg add a
  $ hg commit -m a
  $ touch A
  $ hg add A
  warning: possible case-folding collision for A
  $ hg commit -m A
  $ cd ..

Test disabling the branchcache
  $ hg init branchcache
  $ cd branchcache
  $ cat >> .hg/hgrc <<EOF
  > [extensions]
  > blackbox=
  > strip=
  > EOF
  $ echo a > a
  $ hg commit -Aqm a
  $ hg blackbox
  *> commit -Aqm a (glob)
  *> updated served branch cache in * seconds (glob)
  *> wrote served branch cache with 1 labels and 1 nodes (glob)
  *> commit -Aqm a exited 0 after * seconds (glob)
  $ hg strip -q -r . -k
  $ rm .hg/blackbox.log
  $ rm -rf .hg/cache
  $ hg commit -Aqm a --config perftweaks.disablebranchcache=True
  $ hg blackbox
  *> commit -Aqm a (glob)
  *> perftweaks updated served branch cache (glob)
  *> wrote served branch cache with 1 labels and 1 nodes (glob)
  *> commit -Aqm a --config perftweaks.disablebranchcache=True exited 0 after * seconds (glob)

  $ cd ..

Test changing the delta heuristic
(this isn't a good test, but it executes the code path)
  $ hg init preferdeltaserver
  $ cd preferdeltaserver
  $ touch a && hg commit -Aqm a
  $ touch b && hg commit -Aqm b
  $ cd ..
  $ hg init preferdelta
  $ cd preferdelta
  $ cat >> .hg/hgrc <<EOF
  > [perftweaks]
  > preferdeltas=True
  > EOF
  $ hg pull ../preferdeltaserver
  pulling from ../preferdeltaserver
  requesting all changes
  adding changesets
  adding manifests
  adding file changes
  added 2 changesets with 2 changes to 2 files
  (run 'hg update' to get a working copy)
