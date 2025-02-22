  $ disable treemanifest
  $ setconfig experimental.allowfilepeer=True

This test tries to exercise the ssh functionality with a dummy script

  $ setconfig format.usegeneraldelta=yes
  $ configure dummyssh

Enable narrow-heads on server repos. This test accesses those repos using both
`ssh` and `hg -R`. Enable narrow-heads to get a consistent state.

  $ setconfig experimental.disable-narrow-heads-ssh-server=false

creating 'remote' repo

  $ hg init remote
  $ cd remote
  $ echo this > foo
  $ echo this > fooO
  $ hg ci -A -m "init" foo fooO

configure for serving

  $ setconfig server.uncompressed=true
  $ readconfig <<EOF
  > [hooks]
  > changegroup = sh -c "printenv.py changegroup-in-remote 0 ../dummylog"
  > EOF
  $ cd ..

repo not found error

  $ hg clone ssh://user@dummy/nonexistent local
  remote: abort: repository nonexistent not found!
  abort: no suitable response from remote hg!
  [255]

non-existent absolute path

  $ hg clone ssh://user@dummy/`pwd`/nonexistent local
  remote: abort: repository $TESTTMP/nonexistent not found!
  abort: no suitable response from remote hg!
  [255]

clone remote via stream

  $ hg clone --stream ssh://user@dummy/remote local-stream
  streaming all changes
  5 files to transfer, * of data (glob)
  transferred 398 bytes in 0.0 seconds (389 KB/sec)
  searching for changes
  no changes found
  updating to branch default
  2 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ cd local-stream
  $ hg verify
  warning: verify does not actually check anything in this repo
  $ cd ..

clone bookmarks via stream

  $ hg -R local-stream book mybook
  $ hg clone --stream ssh://user@dummy/local-stream stream2
  streaming all changes
  5 files to transfer, * of data (glob)
  transferred 398 bytes in 0.0 seconds (389 KB/sec)
  searching for changes
  no changes found
  updating to branch default
  2 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ cd stream2
  $ hg book
     mybook                    1160648e36ce
  $ cd ..
  $ rm -rf local-stream stream2

clone remote via pull

  $ hg clone ssh://user@dummy/remote local
  requesting all changes
  adding changesets
  adding manifests
  adding file changes
  added 1 changesets with 2 changes to 2 files
  updating to branch default
  2 files updated, 0 files merged, 0 files removed, 0 files unresolved

verify

  $ cd local
  $ hg verify
  warning: verify does not actually check anything in this repo
  $ cat >> .hg/hgrc <<EOF
  > [hooks]
  > changegroup = sh -c "printenv.py changegroup-in-local 0 ../dummylog"
  > EOF

empty default pull

  $ hg paths
  default = ssh://user@dummy/remote
  $ hg pull -e "$(dummysshcmd)"
  pulling from ssh://user@dummy/remote
  searching for changes
  no changes found

pull from wrong ssh URL

  $ hg pull ssh://user@dummy/doesnotexist
  pulling from ssh://user@dummy/doesnotexist
  remote: abort: repository doesnotexist not found!
  abort: no suitable response from remote hg!
  [255]

local change

  $ echo bleah > foo
  $ hg ci -m "add"

updating rc

  $ echo "default-push = ssh://user@dummy/remote" >> .hg/hgrc
  $ echo "[ui]" >> .hg/hgrc
  $ echo "ssh = $(dummysshcmd)" >> .hg/hgrc

find outgoing

  $ hg out ssh://user@dummy/remote
  comparing with ssh://user@dummy/remote
  searching for changes
  commit:      a28a9d1a809c
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     add
  

find incoming on the remote side

  $ hg incoming -R ../remote ssh://user@dummy/local
  comparing with ssh://user@dummy/local
  searching for changes
  commit:      a28a9d1a809c
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     add
  

find incoming on the remote side (using absolute path)

  $ hg incoming -R ../remote "ssh://user@dummy/`pwd`"
  comparing with ssh://user@dummy/$TESTTMP/local
  searching for changes
  commit:      a28a9d1a809c
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     add
  

push

  $ hg push
  pushing to ssh://user@dummy/remote
  searching for changes
  remote: adding changesets
  remote: adding manifests
  remote: adding file changes
  remote: added 1 changesets with 1 changes to 1 files
  $ cd ../remote

check remote tip

  $ hg tip
  commit:      a28a9d1a809c
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     add
  
  $ hg verify
  warning: verify does not actually check anything in this repo
  $ hg cat -r tip foo
  bleah
  $ echo z > z
  $ hg ci -A -m z z

test pushkeys and bookmarks

  $ cd ../local
  $ hg debugpushkey --config ui.ssh="$(dummysshcmd)" ssh://user@dummy/remote namespaces
  bookmarks	
  namespaces	
  phases	
  $ hg book foo -r 'desc(init)'
  $ hg out -B
  comparing with ssh://user@dummy/remote
  searching for changed bookmarks
     foo                       1160648e36ce
  $ hg push -B foo
  pushing to ssh://user@dummy/remote
  searching for changes
  no changes found
  exporting bookmark foo
  [1]
  $ hg debugpushkey --config ui.ssh="$(dummysshcmd)" ssh://user@dummy/remote bookmarks
  foo	1160648e36cec0054048a7edc4110c6f84fde594
  $ hg book -f foo
  $ hg push --traceback
  pushing to ssh://user@dummy/remote
  searching for changes
  no changes found
  updating bookmark foo
  [1]
  $ hg book -d foo
  $ hg in -B
  comparing with ssh://user@dummy/remote
  searching for changed bookmarks
     foo                       a28a9d1a809c
  $ hg book -f -r 'desc(init)' foo
  $ hg pull -B foo
  pulling from ssh://user@dummy/remote
  no changes found
  updating bookmark foo
  $ hg book -d foo
  $ hg push -B foo
  pushing to ssh://user@dummy/remote
  searching for changes
  no changes found
  deleting remote bookmark foo
  [1]

a bad, evil hook that prints to stdout

  $ cat <<EOF > $TESTTMP/badhook
  > import sys
  > sys.stdout.write("KABOOM\n")
  > EOF

  $ cat <<EOF > $TESTTMP/badpyhook.py
  > import sys
  > def hook(ui, repo, hooktype, **kwargs):
  >     sys.stdout.write("KABOOM IN PROCESS\n")
  >     sys.stdout.flush()
  > EOF

  $ cat <<EOF >> ../remote/.hg/hgrc
  > [hooks]
  > changegroup.stdout = $PYTHON $TESTTMP/badhook
  > changegroup.pystdout = python:$TESTTMP/badpyhook.py:hook
  > EOF
  $ echo r > r
  $ hg ci -A -m z r

push should succeed even though it has an unexpected response

  $ hg push
  pushing to ssh://user@dummy/remote
  searching for changes
  remote: adding changesets
  remote: adding manifests
  remote: adding file changes
  remote: added 1 changesets with 1 changes to 1 files
  remote: KABOOM
  remote: KABOOM IN PROCESS
  $ hg -R ../remote heads
  commit:      1383141674ec
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     z
  
  commit:      6c0482d977a3
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     z
  

clone bookmarks

  $ hg -R ../remote bookmark test
  $ hg -R ../remote bookmarks
   * test                      6c0482d977a3
  $ hg clone ssh://user@dummy/remote local-bookmarks
  requesting all changes
  adding changesets
  adding manifests
  adding file changes
  added 4 changesets with 5 changes to 4 files
  updating to branch default
  3 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ hg -R local-bookmarks bookmarks
     test                      6c0482d977a3

passwords in ssh urls are not supported
(we use a glob here because different Python versions give different
results here)

  $ hg push ssh://user:erroneouspwd@dummy/remote
  pushing to ssh://user:*@dummy/remote (glob)
  abort: password in URL not supported!
  [255]

  $ cd ..

hide outer repo
  $ hg init

Make sure hg is really paranoid in serve --stdio mode. It used to be
possible to get a debugger REPL by specifying a repo named --debugger.
  $ hg -R --debugger serve --stdio
  abort: repository --debugger not found!
  [255]
  $ hg -R --config=ui.debugger=yes serve --stdio
  abort: repository --config=ui.debugger=yes not found!
  [255]
Abbreviations of 'serve' also don't work, to avoid shenanigans.
  $ hg -R narf serv --stdio
  abort: repository narf not found!
  [255]

Test hg-ssh using a helper script that will restore PYTHONPATH (which might
have been cleared by a hg.exe wrapper) and invoke hg-ssh with the right
parameters:

  $ cat > ssh.sh << EOF
  > userhost="\$1"
  > SSH_ORIGINAL_COMMAND="\$2"
  > export SSH_ORIGINAL_COMMAND
  > PYTHONPATH="$PYTHONPATH"
  > export PYTHONPATH
  > hg debugpython -- "$TESTDIR/../contrib/hg-ssh" "$TESTTMP/a repo"
  > EOF


Test hg-ssh in read-only mode:

  $ cat > ssh.sh << EOF
  > userhost="\$1"
  > SSH_ORIGINAL_COMMAND="\$2"
  > export SSH_ORIGINAL_COMMAND
  > PYTHONPATH="$PYTHONPATH"
  > export PYTHONPATH
  > hg debugpython -- "$TESTDIR/../contrib/hg-ssh" --read-only "$TESTTMP/remote"
  > EOF

  $ hg clone -q --ssh "sh ssh.sh" "ssh://user@dummy/$TESTTMP/remote" read-only-local

  $ cd read-only-local
  $ echo "baz" > bar
  $ hg ci -A -m "unpushable commit" bar
  $ hg push -q --ssh "sh ../ssh.sh"
  abort: push failed on remote
  [255]

  $ cd ..

stderr from remote commands should be printed before stdout from local code (issue4336)

  $ hg clone remote stderr-ordering
  updating to branch default
  3 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ cd stderr-ordering
  $ cat >> localwrite.py << EOF
  > from edenscm.mercurial import exchange, extensions
  > 
  > def wrappedpush(orig, repo, *args, **kwargs):
  >     res = orig(repo, *args, **kwargs)
  >     repo.ui.write('local stdout\n')
  >     return res
  > 
  > def extsetup(ui):
  >     extensions.wrapfunction(exchange, 'push', wrappedpush)
  > EOF

  $ cat >> .hg/hgrc << EOF
  > [paths]
  > default-push = ssh://user@dummy/remote
  > [ui]
  > ssh = hg debugpython -- "$TESTDIR/dummyssh"
  > [extensions]
  > localwrite = localwrite.py
  > EOF

  $ echo localwrite > foo
  $ hg commit -m 'testing localwrite'
  $ hg push
  pushing to ssh://user@dummy/remote
  searching for changes
  local stdout
  remote: adding changesets
  remote: adding manifests
  remote: adding file changes
  remote: added 1 changesets with 1 changes to 1 files
  remote: KABOOM
  remote: KABOOM IN PROCESS

debug output

  $ hg pull --debug ssh://user@dummy/remote
  pulling from ssh://user@dummy/remote
  running .* ".*/dummyssh" ['"]user@dummy['"] ('|")hg -R remote serve --stdio('|") (re)
  sending hello command
  sending between command
  remote: 408
  remote: capabilities: lookup changegroupsubset branchmap pushkey known getbundle unbundlehash unbundlereplay batch streamreqs=generaldelta,lz4revlog,revlogv1 stream_option $USUAL_BUNDLE2_CAPS$ unbundle=HG10GZ,HG10BZ,HG10UN
  remote: 1
  query 1; heads
  sending batch command
  searching for changes
  local heads: 2; remote heads: 2 (explicit: 0); initial common: 2
  all remote heads known locally
  no changes found
  sending getbundle command
  bundle2-input-bundle: with-transaction
  bundle2-input-part: "bookmarks" supported
  bundle2-input-part: total payload size 26
  bundle2-input-part: "listkeys" (params: 1 mandatory) supported
  bundle2-input-part: total payload size 45
  bundle2-input-bundle: 1 parts total
  checking for updated bookmarks

  $ cd ..

  $ cat dummylog
  Got arguments 1:user@dummy 2:hg -R nonexistent serve --stdio
  Got arguments 1:user@dummy 2:hg -R $TESTTMP/nonexistent serve --stdio
  Got arguments 1:user@dummy 2:hg -R remote serve --stdio
  Got arguments 1:user@dummy 2:hg -R local-stream serve --stdio
  Got arguments 1:user@dummy 2:hg -R remote serve --stdio
  Got arguments 1:user@dummy 2:hg -R remote serve --stdio
  Got arguments 1:user@dummy 2:hg -R doesnotexist serve --stdio
  Got arguments 1:user@dummy 2:hg -R remote serve --stdio
  Got arguments 1:user@dummy 2:hg -R local serve --stdio
  Got arguments 1:user@dummy 2:hg -R $TESTTMP/local serve --stdio
  Got arguments 1:user@dummy 2:hg -R remote serve --stdio
  changegroup-in-remote hook: HG_BUNDLE2=1 HG_HOOKNAME=changegroup HG_HOOKTYPE=changegroup HG_NODE=a28a9d1a809cab7d4e2fde4bee738a9ede948b60 HG_NODE_LAST=a28a9d1a809cab7d4e2fde4bee738a9ede948b60 HG_SOURCE=serve HG_TXNID=TXN:$ID$ HG_URL=remote:ssh:$LOCALIP
  Got arguments 1:user@dummy 2:hg -R remote serve --stdio
  Got arguments 1:user@dummy 2:hg -R remote serve --stdio
  Got arguments 1:user@dummy 2:hg -R remote serve --stdio
  Got arguments 1:user@dummy 2:hg -R remote serve --stdio
  Got arguments 1:user@dummy 2:hg -R remote serve --stdio
  Got arguments 1:user@dummy 2:hg -R remote serve --stdio
  Got arguments 1:user@dummy 2:hg -R remote serve --stdio
  Got arguments 1:user@dummy 2:hg -R remote serve --stdio
  Got arguments 1:user@dummy 2:hg -R remote serve --stdio
  changegroup-in-remote hook: HG_BUNDLE2=1 HG_HOOKNAME=changegroup HG_HOOKTYPE=changegroup HG_NODE=1383141674ec756a6056f6a9097618482fe0f4a6 HG_NODE_LAST=1383141674ec756a6056f6a9097618482fe0f4a6 HG_SOURCE=serve HG_TXNID=TXN:$ID$ HG_URL=remote:ssh:$LOCALIP
  Got arguments 1:user@dummy 2:hg -R remote serve --stdio
  Got arguments 1:user@dummy 2:hg -R remote serve --stdio
  changegroup-in-remote hook: HG_BUNDLE2=1 HG_HOOKNAME=changegroup HG_HOOKTYPE=changegroup HG_NODE=65c38f4125f9602c8db4af56530cc221d93b8ef8 HG_NODE_LAST=65c38f4125f9602c8db4af56530cc221d93b8ef8 HG_SOURCE=serve HG_TXNID=TXN:$ID$ HG_URL=remote:ssh:$LOCALIP
  Got arguments 1:user@dummy 2:hg -R remote serve --stdio

remote hook failure is attributed to remote

  $ cat > $TESTTMP/failhook << EOF
  > def hook(ui, repo, **kwargs):
  >     ui.write('hook failure!\n')
  >     ui.flush()
  >     return 1
  > EOF

  $ echo "pretxnchangegroup.fail = python:$TESTTMP/failhook:hook" >> remote/.hg/hgrc

  $ hg -q --config ui.ssh="$(dummysshcmd)" clone ssh://user@dummy/remote hookout
  $ cd hookout
  $ touch hookfailure
  $ hg -q commit -A -m 'remote hook failure'
  $ hg --config ui.ssh="$(dummysshcmd)" push
  pushing to ssh://user@dummy/remote
  searching for changes
  remote: pretxnchangegroup.fail hook failed
  remote: adding changesets
  remote: adding manifests
  remote: adding file changes
  remote: added 1 changesets with 1 changes to 1 files
  remote: hook failure!
  abort: push failed on remote
  [255]

abort during pull is properly reported as such

  $ echo morefoo >> ../remote/foo
  $ hg -R ../remote commit --message "more foo to be pulled"
  $ cat >> ../remote/.hg/hgrc << EOF
  > [extensions]
  > crash = ${TESTDIR}/crashgetbundler.py
  > EOF
  $ hg --config ui.ssh="\"$PYTHON\" $TESTDIR/dummyssh" pull
  pulling from ssh://user@dummy/remote
  searching for changes
  remote: abort: this is an exercise
  abort: pull failed on remote
  [255]

abort with no error hint when there is a ssh problem when pulling

  $ hg pull ssh://brokenrepository -e "$(dummysshcmd)"
  pulling from ssh://brokenrepository/
  abort: no suitable response from remote hg* (glob)
  [255]

abort with configured error hint when there is a ssh problem when pulling

  $ hg pull ssh://brokenrepository \
  > --config ui.ssherrorhint="Please see http://company/internalwiki/ssh.html"
  pulling from ssh://brokenrepository/
  abort: no suitable response from remote hg* (glob)
  (Please see http://company/internalwiki/ssh.html)
  [255]

test that custom environment is passed down to ssh executable
  $ cat >>dumpenv <<EOF
  > #! /bin/sh
  > echo \$VAR >&2
  > read hello
  > read between
  > read args
  > EOF
  $ chmod +x dumpenv
  $ hg pull ssh://something --config ui.ssh="./dumpenv"
  pulling from ssh://something/
  remote: 
  abort: no suitable response from remote hg!
  [255]
  $ hg pull ssh://something --config ui.ssh="./dumpenv" --config sshenv.VAR=17
  pulling from ssh://something/
  remote: 17
  abort: no suitable response from remote hg!
  [255]

