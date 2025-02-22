#chg-compatible
  $ setconfig experimental.allowfilepeer=True

Create an extension to test bundle2 with multiple changegroups

  $ cat > bundle2.py <<EOF
  > """
  > """
  > from edenscm.mercurial import changegroup, discovery, exchange
  > 
  > def _getbundlechangegrouppart(bundler, repo, source, bundlecaps=None,
  >                               b2caps=None, heads=None, common=None,
  >                               **kwargs):
  >     # Create two changegroups given the common changesets and heads for the
  >     # changegroup part we are being requested. Use the parent of each head
  >     # in 'heads' as intermediate heads for the first changegroup.
  >     intermediates = [repo[r].p1().node() for r in heads]
  >     outgoing = discovery.outgoing(repo, common, intermediates)
  >     cg = changegroup.makechangegroup(repo, outgoing, '02',
  >                                      source, bundlecaps=bundlecaps)
  >     bundler.newpart('output', data=b'changegroup1')
  >     part = bundler.newpart('changegroup', data=cg.getchunks())
  >     part.addparam('version', '02')
  >     outgoing = discovery.outgoing(repo, common + intermediates, heads)
  >     cg = changegroup.makechangegroup(repo, outgoing, '02',
  >                                      source, bundlecaps=bundlecaps)
  >     bundler.newpart('output', data=b'changegroup2')
  >     part = bundler.newpart('changegroup', data=cg.getchunks())
  >     part.addparam('version', '02')
  > 
  > def _pull(repo, *args, **kwargs):
  >   pullop = _orig_pull(repo, *args, **kwargs)
  >   repo.ui.write('pullop.cgresult is %d\n' % pullop.cgresult)
  >   return pullop
  > 
  > _orig_pull = exchange.pull
  > exchange.pull = _pull
  > exchange.getbundle2partsmapping['changegroup'] = _getbundlechangegrouppart
  > EOF

  $ cat >> $HGRCPATH << EOF
  > [ui]
  > logtemplate={node|short} {phase} {author} {bookmarks} {desc|firstline}
  > EOF

Start with a simple repository with a single commit

  $ hg init repo
  $ cd repo
  $ cat > .hg/hgrc << EOF
  > [extensions]
  > bundle2=$TESTTMP/bundle2.py
  > EOF

  $ echo A > A
  $ hg commit -A -m A -q
  $ cd ..

Clone

  $ hg clone -q repo clone

Add two linear commits

  $ cd repo
  $ echo B > B
  $ hg commit -A -m B -q
  $ echo C > C
  $ hg commit -A -m C -q

  $ cd ../clone
  $ cat >> .hg/hgrc <<EOF
  > [hooks]
  > pretxnchangegroup = sh -c "printenv.py pretxnchangegroup"
  > changegroup = sh -c "printenv.py changegroup"
  > EOF

Pull the new commits in the clone

  $ hg pull
  pulling from $TESTTMP/repo
  searching for changes
  remote: changegroup1
  adding changesets
  adding manifests
  adding file changes
  added 1 changesets with 1 changes to 1 files
  pretxnchangegroup hook: HG_HOOKNAME=pretxnchangegroup HG_HOOKTYPE=pretxnchangegroup HG_NODE=27547f69f25460a52fff66ad004e58da7ad3fb56 HG_NODE_LAST=27547f69f25460a52fff66ad004e58da7ad3fb56 HG_PENDING=$TESTTMP/clone HG_PENDING_METALOG={"$TESTTMP/clone/.hg/store/metalog": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"} HG_SHAREDPENDING=$TESTTMP/clone HG_SOURCE=pull HG_TXNID=TXN:$ID$ HG_URL=file:$TESTTMP/repo
  remote: changegroup2
  adding changesets
  adding manifests
  adding file changes
  added 1 changesets with 1 changes to 1 files
  pretxnchangegroup hook: HG_HOOKNAME=pretxnchangegroup HG_HOOKTYPE=pretxnchangegroup HG_NODE=f838bfaca5c7226600ebcfd84f3c3c13a28d3757 HG_NODE_LAST=f838bfaca5c7226600ebcfd84f3c3c13a28d3757 HG_PENDING=$TESTTMP/clone HG_PENDING_METALOG={"$TESTTMP/clone/.hg/store/metalog": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"} HG_SHAREDPENDING=$TESTTMP/clone HG_SOURCE=pull HG_TXNID=TXN:$ID$ HG_URL=file:$TESTTMP/repo
  pullop.cgresult is 1
  changegroup hook: HG_HOOKNAME=changegroup HG_HOOKTYPE=changegroup HG_NODE=27547f69f25460a52fff66ad004e58da7ad3fb56 HG_NODE_LAST=27547f69f25460a52fff66ad004e58da7ad3fb56 HG_SOURCE=pull HG_TXNID=TXN:$ID$ HG_URL=file:$TESTTMP/repo
  changegroup hook: HG_HOOKNAME=changegroup HG_HOOKTYPE=changegroup HG_NODE=f838bfaca5c7226600ebcfd84f3c3c13a28d3757 HG_NODE_LAST=f838bfaca5c7226600ebcfd84f3c3c13a28d3757 HG_SOURCE=pull HG_TXNID=TXN:$ID$ HG_URL=file:$TESTTMP/repo
  $ hg update
  2 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ hg log -G
  @  f838bfaca5c7 draft test  C
  │
  o  27547f69f254 draft test  B
  │
  o  4a2df7238c3b draft test  A
  
Add more changesets with multiple heads to the original repository

  $ cd ../repo
  $ echo D > D
  $ hg commit -A -m D -q
  $ hg up -r 'desc(B)'
  0 files updated, 0 files merged, 2 files removed, 0 files unresolved
  $ echo E > E
  $ hg commit -A -m E -q
  $ echo F > F
  $ hg commit -A -m F -q
  $ hg up -r 'desc(B)'
  0 files updated, 0 files merged, 2 files removed, 0 files unresolved
  $ echo G > G
  $ hg commit -A -m G -q
  $ hg up -r 'desc(D)'
  2 files updated, 0 files merged, 1 files removed, 0 files unresolved
  $ echo H > H
  $ hg commit -A -m H -q
  $ hg log -G
  @  5cd59d311f65 draft test  H
  │
  │ o  1d14c3ce6ac0 draft test  G
  │ │
  │ │ o  7f219660301f draft test  F
  │ │ │
  │ │ o  8a5212ebc852 draft test  E
  │ ├─╯
  o │  b3325c91a4d9 draft test  D
  │ │
  o │  f838bfaca5c7 draft test  C
  ├─╯
  o  27547f69f254 draft test  B
  │
  o  4a2df7238c3b draft test  A
  
New heads are reported during transfer and properly accounted for in
pullop.cgresult

  $ cd ../clone
  $ hg pull
  pulling from $TESTTMP/repo
  searching for changes
  remote: changegroup1
  adding changesets
  adding manifests
  adding file changes
  added 2 changesets with 2 changes to 2 files
  pretxnchangegroup hook: HG_HOOKNAME=pretxnchangegroup HG_HOOKTYPE=pretxnchangegroup HG_NODE=b3325c91a4d916bcc4cdc83ea3fe4ece46a42f6e HG_NODE_LAST=8a5212ebc8527f9fb821601504794e3eb11a1ed3 HG_PENDING=$TESTTMP/clone HG_PENDING_METALOG={"$TESTTMP/clone/.hg/store/metalog": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"} HG_SHAREDPENDING=$TESTTMP/clone HG_SOURCE=pull HG_TXNID=TXN:$ID$ HG_URL=file:$TESTTMP/repo
  remote: changegroup2
  adding changesets
  adding manifests
  adding file changes
  added 3 changesets with 3 changes to 3 files
  pretxnchangegroup hook: HG_HOOKNAME=pretxnchangegroup HG_HOOKTYPE=pretxnchangegroup HG_NODE=7f219660301fe4c8a116f714df5e769695cc2b46 HG_NODE_LAST=5cd59d311f6508b8e0ed28a266756c859419c9f1 HG_PENDING=$TESTTMP/clone HG_PENDING_METALOG={"$TESTTMP/clone/.hg/store/metalog": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"} HG_SHAREDPENDING=$TESTTMP/clone HG_SOURCE=pull HG_TXNID=TXN:$ID$ HG_URL=file:$TESTTMP/repo
  pullop.cgresult is 1
  changegroup hook: HG_HOOKNAME=changegroup HG_HOOKTYPE=changegroup HG_NODE=b3325c91a4d916bcc4cdc83ea3fe4ece46a42f6e HG_NODE_LAST=8a5212ebc8527f9fb821601504794e3eb11a1ed3 HG_SOURCE=pull HG_TXNID=TXN:$ID$ HG_URL=file:$TESTTMP/repo
  changegroup hook: HG_HOOKNAME=changegroup HG_HOOKTYPE=changegroup HG_NODE=7f219660301fe4c8a116f714df5e769695cc2b46 HG_NODE_LAST=5cd59d311f6508b8e0ed28a266756c859419c9f1 HG_SOURCE=pull HG_TXNID=TXN:$ID$ HG_URL=file:$TESTTMP/repo
  $ hg log -G
  o  5cd59d311f65 draft test  H
  │
  │ o  1d14c3ce6ac0 draft test  G
  │ │
  │ │ o  7f219660301f draft test  F
  │ │ │
  │ │ o  8a5212ebc852 draft test  E
  │ ├─╯
  o │  b3325c91a4d9 draft test  D
  │ │
  @ │  f838bfaca5c7 draft test  C
  ├─╯
  o  27547f69f254 draft test  B
  │
  o  4a2df7238c3b draft test  A
  
Removing a head from the original repository by merging it

  $ cd ../repo
  $ hg merge -r 'desc(G)' -q
  $ hg commit -m Merge
  $ echo I > I
  $ hg commit -A -m H -q
  $ hg log -G
  @  9d18e5bd9ab0 draft test  H
  │
  o    71bd7b46de72 draft test  Merge
  ├─╮
  │ o  5cd59d311f65 draft test  H
  │ │
  o │  1d14c3ce6ac0 draft test  G
  │ │
  │ │ o  7f219660301f draft test  F
  │ │ │
  │ │ o  8a5212ebc852 draft test  E
  ├───╯
  │ o  b3325c91a4d9 draft test  D
  │ │
  │ o  f838bfaca5c7 draft test  C
  ├─╯
  o  27547f69f254 draft test  B
  │
  o  4a2df7238c3b draft test  A
  
Removed heads are reported during transfer and properly accounted for in
pullop.cgresult

  $ cd ../clone
  $ hg pull
  pulling from $TESTTMP/repo
  searching for changes
  remote: changegroup1
  adding changesets
  adding manifests
  adding file changes
  added 1 changesets with 0 changes to 0 files
  pretxnchangegroup hook: HG_HOOKNAME=pretxnchangegroup HG_HOOKTYPE=pretxnchangegroup HG_NODE=71bd7b46de72e69a32455bf88d04757d542e6cf4 HG_NODE_LAST=71bd7b46de72e69a32455bf88d04757d542e6cf4 HG_PENDING=$TESTTMP/clone HG_PENDING_METALOG={"$TESTTMP/clone/.hg/store/metalog": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"} HG_SHAREDPENDING=$TESTTMP/clone HG_SOURCE=pull HG_TXNID=TXN:$ID$ HG_URL=file:$TESTTMP/repo
  remote: changegroup2
  adding changesets
  adding manifests
  adding file changes
  added 1 changesets with 1 changes to 1 files
  pretxnchangegroup hook: HG_HOOKNAME=pretxnchangegroup HG_HOOKTYPE=pretxnchangegroup HG_NODE=9d18e5bd9ab09337802595d49f1dad0c98df4d84 HG_NODE_LAST=9d18e5bd9ab09337802595d49f1dad0c98df4d84 HG_PENDING=$TESTTMP/clone HG_PENDING_METALOG={"$TESTTMP/clone/.hg/store/metalog": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"} HG_SHAREDPENDING=$TESTTMP/clone HG_SOURCE=pull HG_TXNID=TXN:$ID$ HG_URL=file:$TESTTMP/repo
  pullop.cgresult is 1
  changegroup hook: HG_HOOKNAME=changegroup HG_HOOKTYPE=changegroup HG_NODE=71bd7b46de72e69a32455bf88d04757d542e6cf4 HG_NODE_LAST=71bd7b46de72e69a32455bf88d04757d542e6cf4 HG_SOURCE=pull HG_TXNID=TXN:$ID$ HG_URL=file:$TESTTMP/repo
  changegroup hook: HG_HOOKNAME=changegroup HG_HOOKTYPE=changegroup HG_NODE=9d18e5bd9ab09337802595d49f1dad0c98df4d84 HG_NODE_LAST=9d18e5bd9ab09337802595d49f1dad0c98df4d84 HG_SOURCE=pull HG_TXNID=TXN:$ID$ HG_URL=file:$TESTTMP/repo
  $ hg log -G
  o  9d18e5bd9ab0 draft test  H
  │
  o    71bd7b46de72 draft test  Merge
  ├─╮
  │ o  5cd59d311f65 draft test  H
  │ │
  o │  1d14c3ce6ac0 draft test  G
  │ │
  │ │ o  7f219660301f draft test  F
  │ │ │
  │ │ o  8a5212ebc852 draft test  E
  ├───╯
  │ o  b3325c91a4d9 draft test  D
  │ │
  │ @  f838bfaca5c7 draft test  C
  ├─╯
  o  27547f69f254 draft test  B
  │
  o  4a2df7238c3b draft test  A
  
