# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.

  $ . "${TEST_FIXTURES}/library.sh"
  $ export ENABLE_BOOKMARK_CACHE=1
  $ REPOTYPE="blob_files"
  $ setup_common_config $REPOTYPE
  $ GIT_REPO_ORIGIN="${TESTTMP}/origin/repo-git"
  $ GIT_REPO="${TESTTMP}/repo-git"

# Setup git repository
  $ mkdir -p "$GIT_REPO_ORIGIN"
  $ cd "$GIT_REPO_ORIGIN"
  $ git init -q
  $ echo "this is fileA" > fileA
  $ git add fileA
  $ git commit -qam "Add fileA"
  $ git tag -a -m"new tag" first_tag
  $ echo "this is fileA.1" > fileA
  $ echo "this is fileB" > fileB
  $ git add .
  $ git commit -qam "Modified fileA -> fileA.1, Add fileB"
  $ git tag -a empty_tag -m ""
  $ cd "$TESTTMP"
  $ git clone "$GIT_REPO_ORIGIN"
  Cloning into 'repo-git'...
  done.

# Capture all the known Git objects from the repo
  $ cd $GIT_REPO
  $ current_head=$(git rev-parse HEAD)
  $ git rev-list --objects --all | git cat-file --batch-check='%(objectname) %(objecttype) %(rest)' | sort > $TESTTMP/object_list

# Import it into Mononoke
  $ cd "$TESTTMP"
  $ quiet gitimport "$GIT_REPO" --derive-hg --generate-bookmarks full-repo

# Start up the Mononoke Git Service
  $ mononoke_git_service
# Clone the Git repo from Mononoke
  $ quiet git_client clone $MONONOKE_GIT_SERVICE_BASE_URL/$REPONAME.git

# Verify that we get the same Git repo back that we started with
  $ cd $REPONAME  
  $ git rev-list --objects --all | git cat-file --batch-check='%(objectname) %(objecttype) %(rest)' | sort > $TESTTMP/new_object_list
  $ diff -w $TESTTMP/new_object_list $TESTTMP/object_list

# Add more commits to the original git repo
  $ cd $GIT_REPO_ORIGIN
  $ echo "this is fileC" > fileC
  $ git add fileC
  $ git commit -qam "Add fileC"
  $ echo "this is fileD" > fileD
  $ git add fileD
  $ git commit -qam "Add fileD"
  $ git tag -a -m "last tag" last_tag

  $ cd "$GIT_REPO"
  $ quiet git pull "$GIT_REPO_ORIGIN"
# Capture all the known Git objects from the repo
  $ cd $GIT_REPO_ORIGIN
  $ git rev-list --objects --all | git cat-file --batch-check='%(objectname) %(objecttype) %(rest)' | sort > $TESTTMP/object_list

# Import the newly added commits to Mononoke
  $ cd "$TESTTMP"
  $ quiet gitimport "$GIT_REPO_ORIGIN" --derive-hg --generate-bookmarks full-repo

# Pull the Git repo from Mononoke
  $ cd $REPONAME
# Wait for the warm bookmark cache to catch up with the latest changes
  $ wait_for_git_bookmark_create refs/tags/last_tag
  $ git_client pull
  remote: Converting HAVE Git commits to Bonsais        
  remote: Converting WANT Git commits to Bonsais        
  remote: Collecting Bonsai commits to send to client        
  remote: Counting number of objects to be sent in packfile        
  remote: Generating trees and blobs stream        
  remote: Generating commits stream        
  remote: Generating tags stream        
  remote: Sending packfile stream        
  From https://localhost:$LOCAL_PORT/repos/git/ro/repo
     db39b4c..356883f  master_bookmark -> origin/master_bookmark
   * [new tag]         last_tag        -> last_tag
  Updating db39b4c..356883f
  Fast-forward
   fileC | 1 +
   fileD | 1 +
   2 files changed, 2 insertions(+)
   create mode 100644 fileC
   create mode 100644 fileD
# Verify that we get the same Git repo back that we started with
  $ git rev-list --objects --all | git cat-file --batch-check='%(objectname) %(objecttype) %(rest)' | sort > $TESTTMP/new_object_list
  $ diff -w $TESTTMP/new_object_list $TESTTMP/object_list
 
