#chg-compatible

  $ configure modern
  $ configure mutation
  $ setconfig diff.git=True

Test adding, modifying, removing, and renaming files in amend.
  $ newrepo
  $ echo foo > foo
  $ echo bar > bar
  $ echo baz > baz
  $ hg ci -m "one" -Aq
  $ echo qux > qux
  $ hg ci -m "two" -Aq
  $ hg mv foo foo_renamed
  $ hg rm bar
  $ echo morebaz >> baz
  $ echo new > new
  $ hg add new
  $ hg amend --to .^
  $ hg log -G -vp -T "{desc} {join(extras, ' ')} {mutations} {node|short}"
  @  two branch=default mutdate=0 0 mutop=rebase mutpred=hg/7d3606fd19e3e6bb309681ed5af095d173314ab5 mutuser=test rebase_source=7d3606fd19e3e6bb309681ed5af095d173314ab5  01f78cddc939diff --git a/qux b/qux
  │  new file mode 100644
  │  --- /dev/null
  │  +++ b/qux
  │  @@ -0,0 +1,1 @@
  │  +qux
  │
  o  one amend_source=c5580d2879b81dcc2465e5dde5f0ee86218c0dc0 branch=default mutdate=0 0 mutop=amend mutpred=hg/c5580d2879b81dcc2465e5dde5f0ee86218c0dc0 mutuser=test  a48652e46d72diff --git a/baz b/baz
     new file mode 100644
     --- /dev/null
     +++ b/baz
     @@ -0,0 +1,2 @@
     +baz
     +morebaz
     diff --git a/foo_renamed b/foo_renamed
     new file mode 100644
     --- /dev/null
     +++ b/foo_renamed
     @@ -0,0 +1,1 @@
     +foo
     diff --git a/new b/new
     new file mode 100644
     --- /dev/null
     +++ b/new
     @@ -0,0 +1,1 @@
     +new
  







Test removing, modifying and renaming files in subsequent commit.
  $ newrepo
  $ echo foo > foo
  $ echo bar > bar
  $ echo baz > baz
  $ hg ci -m "one" -Aq
  $ echo two > two
  $ hg ci -m "two" -Aq
  $ echo three > three
  $ hg ci -m "three" -Aq
  $ hg mv foo foo_renamed
  $ hg rm bar
  $ echo morebaz >> baz
  $ hg amend --to 'desc(two)'
  $ hg log -G -vp -T "{desc} {join(extras, ' ')} {mutations} {node|short}"
  @  three branch=default mutdate=0 0 mutop=rebase mutpred=hg/0b73272bdcf2bf38c71192959d0e3f750de85ea0 mutuser=test rebase_source=0b73272bdcf2bf38c71192959d0e3f750de85ea0  7b8c275b5725diff --git a/three b/three
  │  new file mode 100644
  │  --- /dev/null
  │  +++ b/three
  │  @@ -0,0 +1,1 @@
  │  +three
  │
  o  two amend_source=fbe2abd632c8512c5496e98dedd5cddb43126c37 branch=default mutdate=0 0 mutop=amend mutpred=hg/fbe2abd632c8512c5496e98dedd5cddb43126c37 mutuser=test  9c63fb016abfdiff --git a/bar b/bar
  │  deleted file mode 100644
  │  --- a/bar
  │  +++ /dev/null
  │  @@ -1,1 +0,0 @@
  │  -bar
  │  diff --git a/baz b/baz
  │  --- a/baz
  │  +++ b/baz
  │  @@ -1,1 +1,2 @@
  │   baz
  │  +morebaz
  │  diff --git a/foo b/foo_renamed
  │  rename from foo
  │  rename to foo_renamed
  │  diff --git a/two b/two
  │  new file mode 100644
  │  --- /dev/null
  │  +++ b/two
  │  @@ -0,0 +1,1 @@
  │  +two
  │
  o  one branch=default  c5580d2879b8diff --git a/bar b/bar
     new file mode 100644
     --- /dev/null
     +++ b/bar
     @@ -0,0 +1,1 @@
     +bar
     diff --git a/baz b/baz
     new file mode 100644
     --- /dev/null
     +++ b/baz
     @@ -0,0 +1,1 @@
     +baz
     diff --git a/foo b/foo
     new file mode 100644
     --- /dev/null
     +++ b/foo
     @@ -0,0 +1,1 @@
     +foo
  







Test three way merge during rebase.
  $ newrepo
  $ printf "one\n\ntwo\n\nthree\n" > foo
  $ hg ci -m "one" -Aq
  $ printf "one\n\ntwo\n\nfour\n" > foo
  $ hg ci -m "two"
  $ printf "five\n\ntwo\n\nfour\n" > foo
  $ hg amend --to .^
  merging foo
  $ hg log -G -vp -T "{desc} {node|short}"
  @  two 706f867d1a33diff --git a/foo b/foo
  │  --- a/foo
  │  +++ b/foo
  │  @@ -2,4 +2,4 @@
  │
  │   two
  │
  │  -three
  │  +four
  │
  o  one 997db81b26b2diff --git a/foo b/foo
     new file mode 100644
     --- /dev/null
     +++ b/foo
     @@ -0,0 +1,5 @@
     +five
     +
     +two
     +
     +three
  









Test replacing file with directory.
  $ newrepo
  $ echo foo > foo
  $ hg ci -m "one" -Aq
  $ echo bar > bar
  $ hg ci -m "two" -Aq
  $ hg rm foo
  $ mkdir foo
  $ echo foo > foo/foo
  $ hg add foo/foo
  $ hg amend --to .^
  $ hg log -G -vp -T "{desc}\n"
  @  two
  │  diff --git a/bar b/bar
  │  new file mode 100644
  │  --- /dev/null
  │  +++ b/bar
  │  @@ -0,0 +1,1 @@
  │  +bar
  │
  o  one
     diff --git a/foo/foo b/foo/foo
     new file mode 100644
     --- /dev/null
     +++ b/foo/foo
     @@ -0,0 +1,1 @@
     +foo
  









Test replacing file with symlink.
  $ newrepo
  $ echo foo > foo
  $ hg ci -m "one" -Aq
  $ echo bar > bar
  $ hg ci -m "two" -Aq
  $ ln -sf bar foo
  $ hg amend --to .^
  $ hg log -G -vp -T "{desc} {node|short}"
  @  two 6e7cc816fa6fdiff --git a/bar b/bar
  │  new file mode 100644
  │  --- /dev/null
  │  +++ b/bar
  │  @@ -0,0 +1,1 @@
  │  +bar
  │
  o  one 7b9a757afa3cdiff --git a/foo b/foo
     new file mode 120000
     --- /dev/null
     +++ b/foo
     @@ -0,0 +1,1 @@
     +bar
     \ No newline at end of file
  





Test replacing file with symlink in subsequent commit.
  $ newrepo
  $ echo foo > foo
  $ hg ci -m "one" -Aq
  $ echo bar > bar
  $ hg ci -m "two" -Aq
  $ ln -sf bar foo
  $ hg amend --to .
  $ hg log -G -vp -T "{desc} {node|short}"
  @  two d17906564d89diff --git a/bar b/bar
  │  new file mode 100644
  │  --- /dev/null
  │  +++ b/bar
  │  @@ -0,0 +1,1 @@
  │  +bar
  │  diff --git a/foo b/foo
  │  old mode 100644
  │  new mode 120000
  │  --- a/foo
  │  +++ b/foo
  │  @@ -1,1 +1,1 @@
  │  -foo
  │  +bar
  │  \ No newline at end of file
  │
  o  one 0174aede5e86diff --git a/foo b/foo
     new file mode 100644
     --- /dev/null
     +++ b/foo
     @@ -0,0 +1,1 @@
     +foo
  






Test renaming a file modified by later commit (not supported).
  $ newrepo
  $ echo foo > foo
  $ hg ci -m "one" -Aq
  $ echo bar >> foo
  $ hg ci -m "two"
  $ hg mv foo bar
  $ hg amend --to .^
  abort: amend would conflict in foo
  [255]
  $ hg status
  A bar
  R foo
  $ hg log -G -vp -T "{desc} {node|short}"
  @  two 7fa82f87fe73diff --git a/foo b/foo
  │  --- a/foo
  │  +++ b/foo
  │  @@ -1,1 +1,2 @@
  │   foo
  │  +bar
  │
  o  one 0174aede5e86diff --git a/foo b/foo
     new file mode 100644
     --- /dev/null
     +++ b/foo
     @@ -0,0 +1,1 @@
     +foo
  









Test conflict during initial patch.
  $ newrepo
  $ echo foo > foo
  $ hg ci -m "one" -Aq
  $ echo bar > foo
  $ hg ci -m "two"
  $ echo baz > foo
  $ hg amend --to .^
  patching file foo
  Hunk #1 FAILED at 0
  abort: amend would conflict in foo
  [255]
  $ hg status
  M foo
  $ hg log -G -vp -T "{desc} {node|short}"
  @  two 171ec227cf58diff --git a/foo b/foo
  │  --- a/foo
  │  +++ b/foo
  │  @@ -1,1 +1,1 @@
  │  -foo
  │  +bar
  │
  o  one 0174aede5e86diff --git a/foo b/foo
     new file mode 100644
     --- /dev/null
     +++ b/foo
     @@ -0,0 +1,1 @@
     +foo
  









Test conflict during rebase.
  $ newrepo
  $ echo foo > foo
  $ hg ci -m "one" -Aq
  $ echo bar > foo
  $ hg ci -m "two"
  $ echo foo > foo
  $ hg ci -m "three"
  $ echo baz > foo
  $ hg amend --to 'desc(one)'
  merging foo
  abort: amend would conflict in foo
  [255]
  $ hg status
  M foo
  $ hg log -G -vp -T "{desc} {node|short}"
  @  three f2c76c6d797ediff --git a/foo b/foo
  │  --- a/foo
  │  +++ b/foo
  │  @@ -1,1 +1,1 @@
  │  -bar
  │  +foo
  │
  o  two 171ec227cf58diff --git a/foo b/foo
  │  --- a/foo
  │  +++ b/foo
  │  @@ -1,1 +1,1 @@
  │  -foo
  │  +bar
  │
  o  one 0174aede5e86diff --git a/foo b/foo
     new file mode 100644
     --- /dev/null
     +++ b/foo
     @@ -0,0 +1,1 @@
     +foo
  






Test amending when target commit has other children.
  $ newrepo
  $ echo foo > foo
  $ hg ci -m "root" -Aq
  $ echo bar > bar
  $ hg ci -m "a" -Aq
  $ hg up 'desc(root)'
  0 files updated, 0 files merged, 1 files removed, 0 files unresolved
  $ echo baz > baz
  $ hg ci -m "b" -Aq
  $ echo more >> foo
  $ hg amend --to 'desc(root)'
  $ hg log -G -vp -T "{desc} {node|short}"
  @  b 016648f10c01diff --git a/baz b/baz
  │  new file mode 100644
  │  --- /dev/null
  │  +++ b/baz
  │  @@ -0,0 +1,1 @@
  │  +baz
  │
  o  root 41500d1b8742diff --git a/foo b/foo
     new file mode 100644
     --- /dev/null
     +++ b/foo
     @@ -0,0 +1,2 @@
     +foo
     +more
  
  o  a 07a119ce0bd1diff --git a/bar b/bar
  │  new file mode 100644
  │  --- /dev/null
  │  +++ b/bar
  │  @@ -0,0 +1,1 @@
  │  +bar
  │
  x  root e1062ec6bdfadiff --git a/foo b/foo
     new file mode 100644
     --- /dev/null
     +++ b/foo
     @@ -0,0 +1,1 @@
     +foo
  










Test amending a renamed file (don't lose copysource).
  $ newrepo
  $ echo foo > foo
  $ hg ci -m "one" -Aq
  $ hg mv foo bar
  $ hg ci -m "two"
  $ echo foo >> bar
  $ hg amend --to .
  $ hg log -G -vp -T "{desc} {node|short}"
  @  two 1560771da02ediff --git a/foo b/bar
  │  rename from foo
  │  rename to bar
  │  --- a/foo
  │  +++ b/bar
  │  @@ -1,1 +1,2 @@
  │   foo
  │  +foo
  │
  o  one 0174aede5e86diff --git a/foo b/foo
     new file mode 100644
     --- /dev/null
     +++ b/foo
     @@ -0,0 +1,1 @@
     +foo
  



Test rebasing across multiple changes to multiple files.
  $ newrepo
  $ echo baz > baz
  $ hg ci -m "zero" -Aq
  $ echo foo > foo
  $ echo bar > bar
  $ hg ci -m "one" -Aq
  $ echo foo >> foo
  $ echo bar >> bar
  $ hg ci -m "two"
  $ echo foo >> foo
  $ echo bar >> bar
  $ hg ci -m "three"
  $ echo baz >> baz
  $ hg amend --to "desc(zero)"
  $ hg log -G -vp -T "{desc} {node|short}"
  @  three bd9189084dc0diff --git a/bar b/bar
  │  --- a/bar
  │  +++ b/bar
  │  @@ -1,2 +1,3 @@
  │   bar
  │   bar
  │  +bar
  │  diff --git a/foo b/foo
  │  --- a/foo
  │  +++ b/foo
  │  @@ -1,2 +1,3 @@
  │   foo
  │   foo
  │  +foo
  │
  o  two a9827e13655ediff --git a/bar b/bar
  │  --- a/bar
  │  +++ b/bar
  │  @@ -1,1 +1,2 @@
  │   bar
  │  +bar
  │  diff --git a/foo b/foo
  │  --- a/foo
  │  +++ b/foo
  │  @@ -1,1 +1,2 @@
  │   foo
  │  +foo
  │
  o  one 735f48a31286diff --git a/bar b/bar
  │  new file mode 100644
  │  --- /dev/null
  │  +++ b/bar
  │  @@ -0,0 +1,1 @@
  │  +bar
  │  diff --git a/foo b/foo
  │  new file mode 100644
  │  --- /dev/null
  │  +++ b/foo
  │  @@ -0,0 +1,1 @@
  │  +foo
  │
  o  zero 471861caabd6diff --git a/baz b/baz
     new file mode 100644
     --- /dev/null
     +++ b/baz
     @@ -0,0 +1,2 @@
     +baz
     +baz
  



Test various error cases.
  $ newrepo
  $ echo foo > foo
  $ hg ci -m "one" -Aq
  $ hg debugmakepublic .
  $ echo bar > bar
  $ hg ci -m "two" -Aq
  $ echo more >> bar
  $ hg amend --to .^
  abort: cannot amend public changesets
  [255]
  $ hg amend --to banana
  abort: unknown revision 'banana'!
  [255]
  $ hg revert bar
  $ hg up .^
  0 files updated, 0 files merged, 1 files removed, 0 files unresolved
  $ echo fork > fork
  $ hg ci -m "fork" -Aq
  $ echo more >> bar
  $ hg amend --to 'desc(two)'
  abort: revision 'desc(two)' is not an ancestor of the working copy
  [255]
  $ hg amend --to '::.'
  abort: '::.' must refer to a single changeset
  [255]
  $ hg amend --to '.' -i
  abort: --to cannot be used with any other options
  [255]
  $ hg amend --to '.' bar
  abort: --to cannot be used with any other options
  [255]
