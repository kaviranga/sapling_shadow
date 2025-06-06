/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

//! Code to deal with deltas received or sent over the wire.

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use bufsize::SizeCounter;
use bytes::Buf;
use bytes::BufMut;
use bytes::BytesMut;
use mercurial_types::delta::Delta;
use mercurial_types::delta::Fragment;

use crate::errors::ErrorKind;

const DELTA_HEADER_LEN: usize = 12;

/// Decodes this delta. Consumes the entire buffer, so accepts a BytesMut.
pub fn decode_delta(buf: BytesMut) -> Result<Delta> {
    let mut buf = buf;
    let mut frags = vec![];
    let mut remaining = buf.len();

    while remaining >= DELTA_HEADER_LEN {
        // Each delta fragment has:
        // ---
        // start offset: i32
        // end offset: i32
        // new length: i32
        // content (new length bytes)
        // ---
        let start = buf.get_i32();
        let end = buf.get_i32();
        let new_len = buf.get_i32();
        // TODO: handle negative values for all the above

        let delta_len = (new_len as usize) + DELTA_HEADER_LEN;
        if remaining < delta_len {
            bail!(ErrorKind::InvalidDelta(format!(
                "expected {} bytes, {} remaining",
                delta_len, remaining
            )));
        }

        frags.push(Fragment {
            start: start as usize,
            end: end as usize,
            // TODO: avoid copies here by switching this to Bytes
            content: buf.split_to(new_len as usize).to_vec(),
        });

        remaining -= delta_len;
    }

    if remaining != 0 {
        bail!(ErrorKind::InvalidDelta(format!(
            "{} trailing bytes in encoded delta",
            remaining
        ),));
    }

    Delta::new(frags).with_context(|| ErrorKind::InvalidDelta("invalid fragment list".into()))
}

#[inline]
pub fn encoded_len(delta: &Delta) -> usize {
    let mut size_counter = SizeCounter::new();
    encode_delta(delta, &mut size_counter);
    size_counter.size()
}

pub fn encode_delta<B: BufMut>(delta: &Delta, out: &mut B) {
    for fragment in delta.fragments() {
        out.put_i32(fragment.start as i32);
        out.put_i32(fragment.end as i32);
        out.put_i32(fragment.content.len() as i32);
        out.put_slice(&fragment.content[..]);
    }
}

#[cfg(test)]
mod test {
    use assert_matches::assert_matches;
    use failure_ext::err_downcast;
    use failure_ext::err_downcast_ref;
    use mononoke_macros::mononoke;
    use quickcheck::quickcheck;

    use super::*;

    #[mononoke::test]
    fn invalid_deltas() {
        let short_delta = BytesMut::from(&b"\0\0\0\0\0\0\0\0\0\0\0\x20"[..]);
        assert_matches!(
            err_downcast!(decode_delta(short_delta).unwrap_err(), err: ErrorKind => err),
            Ok(ErrorKind::InvalidDelta(ref msg))
            if msg == "expected 44 bytes, 12 remaining"
        );

        let short_header = BytesMut::from(&b"\0\0\0\0\0\0"[..]);
        assert_matches!(
            err_downcast!(decode_delta(short_header).unwrap_err(), err: ErrorKind => err),
            Ok(ErrorKind::InvalidDelta(ref msg))
            if msg == "6 trailing bytes in encoded delta"
        );

        // start = 2, end = 0
        let start_after_end = BytesMut::from(&b"\0\0\0\x02\0\0\0\0\0\0\0\0"[..]);
        match decode_delta(start_after_end) {
            Ok(bad) => panic!("unexpected success {:?}", bad),
            Err(err) => match err_downcast_ref!(err, err: ErrorKind => err) {
                Some(&ErrorKind::InvalidDelta(..)) => {}
                Some(bad) => panic!("Bad ErrorKind {:?}", bad),
                None => panic!("Unexpected error {:?}", err),
            },
        }
    }

    quickcheck! {
        fn roundtrip(delta: Delta) -> bool {
            let mut out = vec![];
            encode_delta(&delta, &mut out);
            assert_eq!(encoded_len(&delta), out.len());
            let mut out_bytes = BytesMut::with_capacity(out.len());
            out_bytes.extend_from_slice(&out);
            delta == decode_delta(out_bytes).unwrap()
        }
    }
}
