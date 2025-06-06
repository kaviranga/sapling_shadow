/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#![feature(never_type)]

use std::collections::HashMap;
use std::collections::HashSet;
use std::str::FromStr;

use anyhow::Error;
use bytes::Bytes;
use bytes::BytesMut;
use mercurial_types::FileBytes;
use mercurial_types::HgFileNodeId;
use mercurial_types::NULL_HASH;
use mercurial_types::NonRootMPath;
use mercurial_types::RepoPath;
use mercurial_types::blobs::File;
use mercurial_types::blobs::LFSContent;
use mercurial_types::blobs::META_MARKER;
use mercurial_types::blobs::META_SZ;
use mercurial_types::blobs::filenode_lookup::FileNodeIdPointer;
use mercurial_types::nodehash::HgChangesetId;
use mercurial_types::nodehash::HgNodeHash;
use mercurial_types_mocks::nodehash;
use mercurial_types_mocks::nodehash::FOURS_FNID;
use mercurial_types_mocks::nodehash::ONES_FNID;
use mercurial_types_mocks::nodehash::THREES_FNID;
use mercurial_types_mocks::nodehash::TWOS_FNID;
use mononoke_macros::mononoke;
use mononoke_types::hash::Sha256;
use mononoke_types_mocks::contentid::ONES_CTID;
use mononoke_types_mocks::contentid::TWOS_CTID;
use quickcheck::quickcheck;

#[mononoke::test]
fn nodehash_option() {
    assert_eq!(NULL_HASH.into_option(), None);
    assert_eq!(HgNodeHash::from(None), NULL_HASH);

    assert_eq!(nodehash::ONES_HASH.into_option(), Some(nodehash::ONES_HASH));
    assert_eq!(
        HgNodeHash::from(Some(nodehash::ONES_HASH)),
        nodehash::ONES_HASH
    );
}

#[mononoke::test]
fn nodehash_display_opt() {
    assert_eq!(
        format!("{}", HgNodeHash::display_opt(Some(&nodehash::ONES_HASH))),
        "1111111111111111111111111111111111111111"
    );
    assert_eq!(format!("{}", HgNodeHash::display_opt(None)), "(none)");
}

#[mononoke::test]
fn changeset_id_display_opt() {
    assert_eq!(
        format!("{}", HgChangesetId::display_opt(Some(&nodehash::ONES_CSID))),
        "1111111111111111111111111111111111111111"
    );
    assert_eq!(format!("{}", HgChangesetId::display_opt(None)), "(none)");
}

#[mononoke::test]
fn extract_meta_sz() {
    assert_eq!(META_SZ, META_MARKER.len())
}

#[mononoke::test]
fn extract_meta_0() {
    const DATA: &[u8] = b"foo - no meta";

    assert_eq!(File::extract_meta(DATA), (&[][..], 0));
}

#[mononoke::test]
fn extract_meta_1() {
    const DATA: &[u8] = b"\x01\n\x01\nfoo - empty meta";

    assert_eq!(File::extract_meta(DATA), (&[][..], 4));
}

#[mononoke::test]
fn extract_meta_2() {
    const DATA: &[u8] = b"\x01\nabc\x01\nfoo - some meta";

    assert_eq!(File::extract_meta(DATA), (&b"abc"[..], 7));
}

#[mononoke::test]
fn extract_meta_3() {
    const DATA: &[u8] = b"\x01\nfoo - bad unterminated meta";

    assert_eq!(File::extract_meta(DATA), (&[][..], 2));
}

#[mononoke::test]
fn extract_meta_4() {
    const DATA: &[u8] = b"\x01\n\x01\n\x01\nfoo - bad unterminated meta";

    assert_eq!(File::extract_meta(DATA), (&[][..], 4));
}

#[mononoke::test]
fn extract_meta_5() {
    const DATA: &[u8] = b"\x01\n\x01\n";

    assert_eq!(File::extract_meta(DATA), (&[][..], 4));
}

#[mononoke::test]
fn parse_meta_0() {
    const DATA: &[u8] = b"foo - no meta";

    assert!(File::parse_meta(DATA).is_empty())
}

#[mononoke::test]
fn test_meta_1() {
    const DATA: &[u8] = b"\x01\n\x01\nfoo - empty meta";

    assert!(File::parse_meta(DATA).is_empty())
}

#[mononoke::test]
fn test_meta_2() {
    const DATA: &[u8] = b"\x01\nfoo: bar\x01\nfoo - empty meta";

    let kv: Vec<_> = File::parse_meta(DATA).into_iter().collect();

    assert_eq!(kv, vec![(b"foo".as_ref(), b"bar".as_ref())])
}

#[mononoke::test]
fn test_meta_3() {
    const DATA: &[u8] = b"\x01\nfoo: bar\nblim: blop: blap\x01\nfoo - empty meta";

    let mut kv: Vec<_> = File::parse_meta(DATA).into_iter().collect();
    kv.as_mut_slice().sort();

    assert_eq!(
        kv,
        vec![
            (b"blim".as_ref(), b"blop: blap".as_ref()),
            (b"foo".as_ref(), b"bar".as_ref()),
        ]
    )
}

#[mononoke::test]
fn test_hash_meta_delimiter_only_0() {
    const DELIMITER: &[u8] = b"DELIMITER";
    const DATA: &[u8] = b"DELIMITER\n";

    let mut kv: Vec<_> = File::parse_to_hash_map(DATA, DELIMITER)
        .into_iter()
        .collect();
    kv.as_mut_slice().sort();
    assert_eq!(kv, vec![(b"".as_ref(), b"".as_ref())])
}

#[mononoke::test]
fn test_hash_meta_delimiter_only_1() {
    const DELIMITER: &[u8] = b"DELIMITER";
    const DATA: &[u8] = b"DELIMITER";

    let mut kv: Vec<_> = File::parse_to_hash_map(DATA, DELIMITER)
        .into_iter()
        .collect();
    kv.as_mut_slice().sort();
    assert_eq!(kv, vec![(b"".as_ref(), b"".as_ref())])
}

#[mononoke::test]
fn test_hash_meta_delimiter_short_0() {
    const DELIMITER: &[u8] = b"DELIMITER";
    const DATA: &[u8] = b"DELIM";

    let mut kv: Vec<_> = File::parse_to_hash_map(DATA, DELIMITER)
        .into_iter()
        .collect();
    assert!(kv.as_mut_slice().is_empty())
}

#[mononoke::test]
fn test_hash_meta_delimiter_short_1() {
    const DELIMITER: &[u8] = b"DELIMITER";
    const DATA: &[u8] = b"\n";

    let mut kv: Vec<_> = File::parse_to_hash_map(DATA, DELIMITER)
        .into_iter()
        .collect();
    assert!(kv.as_mut_slice().is_empty())
}

#[mononoke::test]
fn test_parse_to_hash_map_long_delimiter() {
    const DATA: &[u8] = b"x\nfooDELIMITERbar\nfoo1DELIMITERbar1";
    const DELIMITER: &[u8] = b"DELIMITER";
    let mut kv: Vec<_> = File::parse_to_hash_map(DATA, DELIMITER)
        .into_iter()
        .collect();
    kv.as_mut_slice().sort();
    assert_eq!(
        kv,
        vec![
            (b"foo".as_ref(), b"bar".as_ref()),
            (b"foo1".as_ref(), b"bar1".as_ref()),
        ]
    )
}

fn check_meta_roundtrip(file: &[u8], meta: &[u8]) {
    let mut buff = BytesMut::new();
    buff.extend_from_slice(meta);
    buff.extend_from_slice(file);
    let f = File::new(buff.freeze(), None, None);
    assert_eq!(f.content(), file);
    assert_eq!(f.metadata(), meta);
}

#[mononoke::test]
fn generate_metadata_0() {
    const FILE_BYTES: &[u8] = b"foobar";
    let file_bytes = FileBytes(Bytes::from(FILE_BYTES));
    let mut meta: Vec<u8> = vec![];
    File::generate_metadata(None, &file_bytes, &mut meta).expect("Vec::write_all should succeed");
    assert_eq!(meta.as_slice(), &b""[..]);
    check_meta_roundtrip(FILE_BYTES, &meta);

    let mut meta: Vec<u8> = vec![];
    File::generate_metadata(
        Some(&(NonRootMPath::new("foo").unwrap(), nodehash::ONES_FNID)),
        &file_bytes,
        &mut meta,
    )
    .expect("Vec::write_all should succeed");
    assert_eq!(
        meta.as_slice(),
        &b"\x01\ncopy: foo\ncopyrev: 1111111111111111111111111111111111111111\n\x01\n"[..]
    );
    check_meta_roundtrip(FILE_BYTES, &meta);
}

#[mononoke::test]
fn generate_metadata_1() {
    // The meta marker in the beginning should cause metadata to unconditionally be emitted.
    const FILE_BYTES: &[u8] = b"\x01\nfoobar";
    let file_bytes = FileBytes(Bytes::from(FILE_BYTES));
    let mut meta: Vec<u8> = vec![];
    File::generate_metadata(None, &file_bytes, &mut meta).expect("Vec::write_all should succeed");
    assert_eq!(meta.as_slice(), &b"\x01\n\x01\n"[..]);
    check_meta_roundtrip(FILE_BYTES, &meta);

    let mut meta: Vec<u8> = vec![];
    File::generate_metadata(
        Some(&(NonRootMPath::new("foo").unwrap(), nodehash::ONES_FNID)),
        &file_bytes,
        &mut meta,
    )
    .expect("Vec::write_all should succeed");
    assert_eq!(
        meta.as_slice(),
        &b"\x01\ncopy: foo\ncopyrev: 1111111111111111111111111111111111111111\n\x01\n"[..]
    );
    check_meta_roundtrip(FILE_BYTES, &meta);
}

#[mononoke::test]
fn test_get_lfs_hash_map() {
    const DATA: &[u8] = b"version https://git-lfs.github.com/spec/v1\noid sha256:27c0a92fc51290e3227bea4dd9e780c5035f017de8d5ddfa35b269ed82226d97\nsize 17";

    let mut kv: Vec<_> = File::parse_content_to_lfs_hash_map(DATA)
        .into_iter()
        .collect();
    kv.as_mut_slice().sort();

    assert_eq!(
        kv,
        vec![
            (
                b"oid".as_ref(),
                b"sha256:27c0a92fc51290e3227bea4dd9e780c5035f017de8d5ddfa35b269ed82226d97".as_ref(),
            ),
            (b"size".as_ref(), b"17".as_ref()),
            (
                b"version".as_ref(),
                b"https://git-lfs.github.com/spec/v1".as_ref(),
            ),
        ]
    )
}

#[mononoke::test]
fn test_get_lfs_struct_0() {
    let mut kv = HashMap::new();
    kv.insert(
        b"version".as_ref(),
        b"https://git-lfs.github.com/spec/v1".as_ref(),
    );
    kv.insert(
        b"oid".as_ref(),
        b"sha256:27c0a92fc51290e3227bea4dd9e780c5035f017de8d5ddfa35b269ed82226d97".as_ref(),
    );
    kv.insert(b"size".as_ref(), b"17".as_ref());
    let lfs = File::get_lfs_struct(&kv);

    assert_eq!(
        lfs.unwrap(),
        LFSContent::new(
            "https://git-lfs.github.com/spec/v1".to_string(),
            Sha256::from_str("27c0a92fc51290e3227bea4dd9e780c5035f017de8d5ddfa35b269ed82226d97")
                .unwrap(),
            17,
            None,
        )
    )
}

#[mononoke::test]
fn test_get_lfs_struct_wrong_small_sha256() {
    let mut kv = HashMap::new();
    kv.insert(
        b"version".as_ref(),
        b"https://git-lfs.github.com/spec/v1".as_ref(),
    );
    kv.insert(b"oid".as_ref(), b"sha256:123".as_ref());
    kv.insert(b"size".as_ref(), b"17".as_ref());
    let lfs = File::get_lfs_struct(&kv);

    assert!(lfs.is_err())
}

#[mononoke::test]
fn test_get_lfs_struct_wrong_size() {
    let mut kv = HashMap::new();
    kv.insert(
        b"version".as_ref(),
        b"https://git-lfs.github.com/spec/v1".as_ref(),
    );
    kv.insert(
        b"oid".as_ref(),
        b"sha256:27c0a92fc51290e3227bea4dd9e780c5035f017de8d5ddfa35b269ed82226d97".as_ref(),
    );
    kv.insert(b"size".as_ref(), b"wrong_size_length".as_ref());
    let lfs = File::get_lfs_struct(&kv);

    assert!(lfs.is_err())
}

#[mononoke::test]
fn test_get_lfs_struct_non_all_mandatory_fields() {
    let mut kv = HashMap::new();
    kv.insert(
        b"oid".as_ref(),
        b"sha256:27c0a92fc51290e3227bea4dd9e780c5035f017de8d5ddfa35b269ed82226d97".as_ref(),
    );
    let lfs = File::get_lfs_struct(&kv);

    assert!(lfs.is_err())
}

#[mononoke::test]
fn test_roundtrip_lfs_content() {
    let oid = Sha256::from_str("27c0a92fc51290e3227bea4dd9e780c5035f017de8d5ddfa35b269ed82226d97")
        .unwrap();
    let size = 10;

    let generated_file = File::generate_lfs_file(oid, size, None).unwrap();
    let lfs_struct = File::data_only(generated_file).get_lfs_content().unwrap();

    let expected_lfs_struct = LFSContent::new(
        "https://git-lfs.github.com/spec/v1".to_string(),
        oid,
        size,
        None,
    );
    assert_eq!(lfs_struct, expected_lfs_struct)
}

quickcheck! {
    fn copy_info_roundtrip(
        copy_info: Option<(NonRootMPath, HgFileNodeId)>,
        file_bytes: FileBytes
    ) -> bool {
        let mut buf = Vec::new();
        let result = File::generate_metadata(copy_info.as_ref(), &file_bytes, &mut buf)
            .and_then(|_| {
                File::extract_copied_from(&buf)
            });
        match result {
            Ok(out_copy_info) => copy_info == out_copy_info,
            _ => {
                false
            }
        }
    }

    fn lfs_copy_info_roundtrip(
        oid: Sha256,
        size: u64,
        copy_from: Option<(NonRootMPath, HgFileNodeId)>
    ) -> bool {
        let result = File::generate_lfs_file(oid, size, copy_from.clone())
            .and_then(|bytes| File::data_only(bytes).get_lfs_content());

        match result {
            Ok(result) => result.oid() == oid && result.size() == size && result.copy_from() == copy_from,
            _ => false,
        }
    }
}

#[mononoke::test]
fn test_hashes_are_unique() -> Result<(), Error> {
    let mut h = HashSet::new();

    for content_id in [ONES_CTID, TWOS_CTID] {
        for p1 in [Some(ONES_FNID), Some(TWOS_FNID), None] {
            for p2 in [Some(THREES_FNID), Some(FOURS_FNID), None] {
                let path1 = RepoPath::file("path")?
                    .into_mpath()
                    .ok_or_else(|| Error::msg("path1"))?;

                let path2 = RepoPath::file("path/2")?
                    .into_mpath()
                    .ok_or_else(|| Error::msg("path2"))?;

                let path3 = RepoPath::file("path2")?
                    .into_mpath()
                    .ok_or_else(|| Error::msg("path3"))?;

                for copy_path in [path1, path2, path3] {
                    for copy_parent in [ONES_FNID, TWOS_FNID, THREES_FNID] {
                        let copy_info = Some((copy_path.clone(), copy_parent.clone()));

                        let ptr = FileNodeIdPointer::new(&content_id, &copy_info, &p1, &p2);
                        assert!(!h.contains(&ptr), "Duplicate entry: {:?}", ptr);
                        h.insert(ptr);

                        if p1 == p2 {
                            continue;
                        }

                        let ptr = FileNodeIdPointer::new(&content_id, &copy_info, &p2, &p1);
                        assert!(!h.contains(&ptr), "Duplicate entry: {:?}", ptr);
                        h.insert(ptr);
                    }
                }

                let ptr = FileNodeIdPointer::new(&content_id, &None, &p1, &p2);
                assert!(!h.contains(&ptr), "Duplicate entry: {:?}", ptr);
                h.insert(ptr);

                if p1 == p2 {
                    continue;
                }

                let ptr = FileNodeIdPointer::new(&content_id, &None, &p2, &p1);
                assert!(!h.contains(&ptr), "Duplicate entry: {:?}", ptr);
                h.insert(ptr);
            }
        }
    }

    Ok(())
}
