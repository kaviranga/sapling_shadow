# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.

  $ . "${TEST_FIXTURES}/library.sh"

  $ cat >> $HGRCPATH <<EOF
  > [extensions]
  > rebase=
  > pushrebase=
  > EOF

setup configuration

  $ REPOTYPE="blob_files"
  $ REPOID=0 REPONAME=large setup_common_config $REPOTYPE
  $ REPOID=1 REPONAME=small setup_common_config $REPOTYPE
  $ setup_commitsyncmap
  $ setup_configerator_configs

  $ cd "$TESTTMP"
  $ hginit_treemanifest large
  $ cd large
  $ mkdir -p .fbsource-rest/arvr
  $ echo 1 > .fbsource-rest/arvr/1
  $ hg add .fbsource-rest/arvr/1
  $ hg commit -m 'equivalent wc'
  $ LARGE_EQ_WC_HASH="$(hg log -r . -T '{node}')"
  $ hg up -q null
  $ echo 1 > 1
  $ hg add 1
  $ hg ci -m 'not sync candidate'
  $ NOT_SYNC_CANDIDATE_HASH="$(hg log -r . -T '{node}')"


  $ cd "$TESTTMP"
  $ hginit_treemanifest small
  $ cd small
  $ mkdir arvr
  $ echo 1 > arvr/1
  $ hg add arvr/1
  $ hg commit -m 'equivalent wc'
  $ SMALL_EQ_WC_HASH="$(hg log -r . -T '{node}')"

blobimport hg servers repos into Mononoke repos
  $ cd "$TESTTMP"
  $ REPOID=0 blobimport large/.hg large
  $ REPOID=1 blobimport small/.hg small

Try to insert with invalid version name
  $ mononoke_admin cross-repo --source-repo-name large --target-repo-name small insert equivalent-working-copy \
  > --source-commit-id "$LARGE_EQ_WC_HASH" --target-commit-id "$SMALL_EQ_WC_HASH" --version-name invalid  2>&1 | grep 'invalid version'
  * invalid version does not exist (glob)

Now insert with valid version name
  $ mononoke_admin cross-repo --source-repo-name large --target-repo-name small insert equivalent-working-copy \
  > --source-commit-id "$LARGE_EQ_WC_HASH" --target-commit-id "$SMALL_EQ_WC_HASH" --version-name TEST_VERSION_NAME 2>&1 | grep 'successfully inserted'
  * successfully inserted equivalent working copy (glob)
  $ mononoke_admin cross-repo --source-repo-name large --target-repo-name small map -i "$LARGE_EQ_WC_HASH" 2>&1 | grep EquivalentWorking
  EquivalentWorkingCopyAncestor(ChangesetId(Blake2(a246023ccc3b1dc56076a2524cd644fb4cb4a99ee2141b2277677f9ce82f0f13)), CommitSyncConfigVersion("TEST_VERSION_NAME"))

Now insert not sync candidate entry
  $ mononoke_admin cross-repo --source-repo-name large --target-repo-name small insert not-sync-candidate \
  > --large-commit-id "$NOT_SYNC_CANDIDATE_HASH" --version-name TEST_VERSION_NAME 2>&1 | grep 'successfully inserted'
  * successfully inserted not sync candidate entry (glob)
  $ mononoke_admin cross-repo --source-repo-name large --target-repo-name small map -i "$NOT_SYNC_CANDIDATE_HASH" 2>&1 | grep NotSyncCandidate
  NotSyncCandidate(CommitSyncConfigVersion("TEST_VERSION_NAME"))
