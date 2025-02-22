# Copyright (c) Facebook, Inc. and its affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.

  $ . "${TEST_FIXTURES}/library.sh"

Set up local hgrc and Mononoke config.
  $ setup_common_config
  $ setup_configerator_configs
  $ cd $TESTTMP

Initialize test repo.
  $ hginit_treemanifest repo-hg
  $ cd repo-hg
  $ setup_hg_server

Populate test repo
  $ echo "test content" > test.txt
  $ hg commit -Aqm "add test.txt"
  $ TEST_FILENODE=$(hg manifest --debug | grep test.txt | awk '{print $1}')
  $ hg cp test.txt copy.txt
  $ hg commit -Aqm "copy test.txt to test2.txt"
  $ COPY_FILENODE=$(hg manifest --debug | grep copy.txt | awk '{print $1}')

Blobimport test repo.
  $ cd ..
  $ blobimport repo-hg/.hg repo

Start up EdenAPI server.
  $ setup_mononoke_config
  $ start_edenapi_server

Create and send file request.
  $ edenapi_make_req file > req.cbor <<EOF
  > {
  >   "keys": [
  >     ["copy.txt", "$COPY_FILENODE"]
  >   ],
  >   "reqs": [
  >     [["test.txt", "$TEST_FILENODE"], {"aux_data": true, "content": true}]
  >   ]
  > }
  > EOF
  Reading from stdin
  Generated request: WireFileRequest {
      keys: [
          WireKey {
              path: WireRepoPathBuf(
                  "copy.txt",
              ),
              hgid: WireHgId("17b8d4e3bafd4ec4812ad7c930aace9bf07ab033"),
          },
      ],
      reqs: [
          WireFileSpec {
              key: WireKey {
                  path: WireRepoPathBuf(
                      "test.txt",
                  ),
                  hgid: WireHgId("186cafa3319c24956783383dc44c5cbc68c5a0ca"),
              },
              attrs: WireFileAttributes {
                  content: true,
                  aux_data: true,
              },
          },
      ],
  }
  $ sslcurl -s "$EDENAPI_URI/repo/files" -d@req.cbor > res.cbor

Check files in response.
  $ edenapi_read_res file ls res.cbor
  Reading from file: "res.cbor"
  17b8d4e3bafd4ec4812ad7c930aace9bf07ab033 copy.txt
  186cafa3319c24956783383dc44c5cbc68c5a0ca test.txt

Verify that filenode hashes match contents.
  $ edenapi_read_res file check res.cbor
  Reading from file: "res.cbor"

Examine file data.
  $ edenapi_read_res file cat res.cbor -p test.txt -h $TEST_FILENODE
  Reading from file: "res.cbor"
  test content

Examine entry structure.
  $ edenapi_read_res file cat res.cbor --debug -p test.txt -h $TEST_FILENODE
  Reading from file: "res.cbor"
  FileEntry { key: Key { path: RepoPathBuf("test.txt"), hgid: HgId("186cafa3319c24956783383dc44c5cbc68c5a0ca") }, parents: None, content: Some(FileContent { hg_file_blob: b"test content\n", metadata: Metadata { size: None, flags: None } }), aux_data: Some(FileAuxData { total_size: 13, content_id: ContentId("888dcf533a354c23e4bf67e1ada984d96bb1089b0c3c03f4c2cb773709e7aa42"), sha1: Sha1("4fe2b8dd12cd9cd6a413ea960cd8c09c25f19527"), sha256: Sha256("a1fff0ffefb9eace7230c24e50731f0a91c62f9cefdfe77121c2f607125dffae") }) }

Note that copyinfo header is present for the copied file.
  $ edenapi_read_res file cat res.cbor -p copy.txt -h $COPY_FILENODE
  Reading from file: "res.cbor"
  \x01 (esc)
  copy: test.txt
  copyrev: 186cafa3319c24956783383dc44c5cbc68c5a0ca
  \x01 (esc)
  test content
