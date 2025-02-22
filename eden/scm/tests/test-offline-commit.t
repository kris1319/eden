#chg-compatible
  $ setconfig experimental.allowfilepeer=True

  $ configure modern
  $ setconfig paths.default=test:e1 ui.ssh=false

Prepare Repo:

  $ newremoterepo
  $ setconfig paths.default=test:e1
  $ drawdag << 'EOS'
  > E
  > |
  > D
  > |
  > C
  > |
  > B
  > |
  > A
  > EOS

  $ hg push -r $E --to master --create -q

Clone the lazy repo:

  $ hg clone -U --shallow test:e1 --config remotefilelog.reponame=x --config clone.force-edenapi-clonedata=1 cloned1 -q
  $ cd cloned1

Commit and edit on top of B:

  $ LOG=dag::protocol=debug,checkout::prefetch=debug hg up $B -q
  DEBUG dag::protocol: resolve names [112478962961147124edd43549aedd1a335e44bf] remotely
  DEBUG dag::protocol: resolve ids [2] remotely
  DEBUG checkout::prefetch: children of 112478962961147124edd43549aedd1a335e44bf: ['26805aba1e600a82e93661149f2313866a221a7b']
  DEBUG dag::protocol: resolve ids [0] remotely
  $ touch B1
  $ LOG=dag::protocol=debug hg commit -Am B1 B1

  $ LOG=dag::protocol=debug hg metaedit -m B11
