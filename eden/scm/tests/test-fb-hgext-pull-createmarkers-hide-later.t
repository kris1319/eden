#chg-compatible
  $ setconfig experimental.allowfilepeer=True


Setup

  $ configure mutation-norecord dummyssh
  $ enable amend pullcreatemarkers pushrebase rebase remotenames
  $ setconfig ui.username="nobody <no.reply@fb.com>" experimental.rebaseskipobsolete=true
  $ setconfig remotenames.allownonfastforward=true

Test that hg pull creates obsolescence markers for landed diffs
  $ hg init server
  $ mkcommit() {
  >    echo "$1" > "$1"
  >    hg add "$1"
  >    echo "add $1" > msg
  >    echo "" >> msg
  >    [ -z "$2" ] || echo "Differential Revision: https://phabricator.fb.com/D$2" >> msg
  >    hg ci -l msg
  > }
  $ land_amend() {
  >    hg log -r. -T'{desc}\n' > msg
  >    echo "Reviewed By: someone" >> msg
  >    hg ci --amend -l msg
  > }

Set up server repository

  $ cd server
  $ mkcommit initial
  $ mkcommit secondcommit
  $ hg book master
  $ cd ..

Set up a client repository, and work on 3 diffs

  $ hg clone ssh://user@dummy/server client -q
  $ cd client
  $ mkcommit b 123 # 123 is the phabricator rev number (see function above)
  $ mkcommit c 124
  $ mkcommit d 131
  $ hg log -G -T '"{desc}" {remotebookmarks}' -r 'all()'
  @  "add d
  │
  │  Differential Revision: https://phabricator.fb.com/D131"
  o  "add c
  │
  │  Differential Revision: https://phabricator.fb.com/D124"
  o  "add b
  │
  │  Differential Revision: https://phabricator.fb.com/D123"
  o  "add secondcommit" default/master
  │
  o  "add initial"
  

Now land the first two diff, but with amended commit messages, as would happen
when a diff is landed with landcastle.

  $ hg update -r 11b76ecbf1d49ab485207f46d8c45ee8c96b1bfb
  0 files updated, 0 files merged, 3 files removed, 0 files unresolved
  $ hg graft -r 948715751816b5aaf59c890f413d3b4c89008f12
  grafting 948715751816 "add b"
  $ land_amend
  $ hg graft -r 0e229072f72376ff68c3ead4de01e8b8888e1e50
  grafting 0e229072f723 "add c"
  $ land_amend
  $ hg push -r . --to master
  pushing rev cc68f5e5f8d6 to destination ssh://user@dummy/server bookmark master
  searching for changes
  updating bookmark master
  remote: pushing 2 changesets:
  remote:     e0672eeeb97c  add b
  remote:     cc68f5e5f8d6  add c

Strip the commits we just landed.

  $ hg update -r 11b76ecbf1d49ab485207f46d8c45ee8c96b1bfb
  0 files updated, 0 files merged, 2 files removed, 0 files unresolved
  $ hg debugstrip -r e0672eeeb97c5767cc642e702951cfcfa73cdc82

Here pull should now detect commits 2 and 3 as landed, but it won't be able to
hide them since there is a non-hidden successor.

  $ hg pull
  pulling from ssh://user@dummy/server
  searching for changes
  adding changesets
  adding manifests
  adding file changes
  added 2 changesets with 0 changes to 2 files
  $ hg log -G -T '"{desc}" {remotebookmarks}' -r 'all()'
  o  "add c
  │
  │  Differential Revision: https://phabricator.fb.com/D124
  │  Reviewed By: someone" default/master
  o  "add b
  │
  │  Differential Revision: https://phabricator.fb.com/D123
  │  Reviewed By: someone"
  │ o  "add d
  │ │
  │ │  Differential Revision: https://phabricator.fb.com/D131"
  │ x  "add c
  │ │
  │ │  Differential Revision: https://phabricator.fb.com/D124"
  │ x  "add b
  ├─╯
  │    Differential Revision: https://phabricator.fb.com/D123"
  @  "add secondcommit"
  │
  o  "add initial"
  
  $ hg log -T '{node}\n' -r 'allsuccessors(948715751816b5aaf59c890f413d3b4c89008f12)'
  e0672eeeb97c5767cc642e702951cfcfa73cdc82
  $ hg log -T '{node}\n' -r 'allsuccessors(0e229072f72376ff68c3ead4de01e8b8888e1e50)'
  cc68f5e5f8d6a0aa5683ff6fb1afd15aa95a08b8

Now land the last diff.

  $ hg update -r 'max(desc(add))'
  2 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ hg graft -r e4b5974890c0ceff0317ecbc08ec357613fd01dd
  grafting e4b5974890c0 "add d"
  $ land_amend
  $ hg push -r . --to master
  pushing rev 296f9d37d5c1 to destination ssh://user@dummy/server bookmark master
  searching for changes
  updating bookmark master
  remote: pushing 1 changeset:
  remote:     296f9d37d5c1  add d

And strip the commit we just landed.

  $ hg update -r cc68f5e5f8d6a0aa5683ff6fb1afd15aa95a08b8
  0 files updated, 0 files merged, 1 files removed, 0 files unresolved
  $ hg debugstrip -r 'max(desc(add))'

Here pull should now detect commit 4 has been landed.  It should hide this
commit, and should also hide 3 and 2, which were previously landed, but up
until now had non-hidden successors.

  $ hg pull
  pulling from ssh://user@dummy/server
  searching for changes
  adding changesets
  adding manifests
  adding file changes
  added 1 changesets with 0 changes to 1 files
  $ hg log -G -T '"{desc}" {remotebookmarks}' -r 'all()'
  o  "add d
  │
  │  Differential Revision: https://phabricator.fb.com/D131
  │  Reviewed By: someone" default/master
  @  "add c
  │
  │  Differential Revision: https://phabricator.fb.com/D124
  │  Reviewed By: someone"
  o  "add b
  │
  │  Differential Revision: https://phabricator.fb.com/D123
  │  Reviewed By: someone"
  o  "add secondcommit"
  │
  o  "add initial"
  
