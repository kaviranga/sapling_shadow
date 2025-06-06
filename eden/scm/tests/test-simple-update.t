#require no-eden

  $ hg init test
  $ cd test
  $ echo foo>foo
  $ hg addremove
  adding foo
  $ hg commit -m "1"

  $ hg verify
  warning: verify does not actually check anything in this repo

  $ hg clone . ../branch
  updating to tip
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ cd ../branch
  $ hg co tip
  0 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ echo bar>>foo
  $ hg commit -m "2"
  $ hg whereami
  30aff43faee11c21aa9036768ad78cc32a171e06

  $ cd ../test

  $ hg pull ../branch -r 30aff43faee11c21aa9036768ad78cc32a171e06
  pulling from ../branch
  searching for changes
  adding changesets
  adding manifests
  adding file changes

  $ hg verify
  warning: verify does not actually check anything in this repo

  $ hg co tip
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved

  $ cat foo
  foo
  bar

  $ hg manifest --debug
  6f4310b00b9a147241b071a60c28a650827fb03d 644   foo

update to rev 0 with a date

  $ hg upd -d foo 0
  abort: you can't specify a revision and a date
  [255]

  $ cd ..

update with worker processes

#if no-windows

  $ cat <<EOF > forceworker.py
  > from sapling import extensions, worker
  > def nocost(orig, ui, costperop, nops):
  >     return worker._numworkers(ui) > 1
  > def uisetup(ui):
  >     extensions.wrapfunction(worker, 'worthwhile', nocost)
  > EOF

  $ hg init worker
  $ cd worker
  $ cat <<EOF >> .hg/hgrc
  > [extensions]
  > forceworker = $TESTTMP/forceworker.py
  > [worker]
  > numcpus = 4
  > EOF
  $ for i in `seq 1 100`; do
  >   echo $i > $i
  > done
  $ hg ci -qAm 'add 100 files'

  $ hg goto null
  0 files updated, 0 files merged, 100 files removed, 0 files unresolved
  $ hg goto tip
  100 files updated, 0 files merged, 0 files removed, 0 files unresolved

  $ cd ..

#endif
