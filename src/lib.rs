use std::cmp::Ordering;
use std::iter::once;

use either::Either;
use fasthash::murmur3::Murmur3Hasher_x86_32;
use itertools::{EitherOrBoth, EitherOrBoth::*, Itertools};
use perfect_hash::PerfectHasher32;
use wu_diff::*;

trait HashDiff<T, I, D> {
    fn hash_diff_vec(self, new: I) -> Vec<D>;
}

trait LineDiff {
    fn lines_hash_diff(self, new: &str) -> Vec<DiffResult>;
}

impl LineDiff for &str {
    fn lines_hash_diff(self, new: &str) -> Vec<DiffResult> {
        let old = self.lines();
        let new = new.lines();

        let (fw_eq_thru, fw) = {
            let mut fw = old.clone().zip_longest(new.clone()).enumerate();
            let mut pre: Option<(usize, EitherOrBoth<&str, &str>)> = None;

            loop {
                match fw.next() {
                    None => {
                        if pre.is_some() {
                            break (pre, fw);
                        } else {
                            break (None, fw);
                        }
                    }
                    Some((i, Both(o, n))) => {
                        pre = Some((i, Both(o, n)));

                        if o == n {
                            continue;
                        } else {
                            break (pre, fw);
                        }
                    }
                    m => break (m, fw),
                }
            }
        };

        if fw_eq_thru.is_none() {
            // both old and new are empty
            return vec![];
        }

        let (fw_index, fw_item) = fw_eq_thru.unwrap();

        if fw_item.is_just_left() || fw_item.is_just_right() {
            // TODO keep track of length individually.
            // return added overall fw.next
            unimplemented!()
        }

        let (bw_eq_thru, bw) = {
            let mut bw = old.rev().zip_longest(new.rev()).enumerate();
            let mut pre: Option<(usize, EitherOrBoth<&str, &str>)> = None;

            loop {
                match bw.next() {
                    None => {
                        if pre.is_some() {
                            break (pre, bw);
                        } else {
                            break (None, bw);
                        }
                    }
                    Some((i, Both(o, n))) => {
                        pre = Some((i, Both(o, n)));

                        if o == n {
                            continue;
                        } else {
                            break (pre, bw);
                        }
                    }
                    m => break (m, bw),
                }
            }
        };

        // Early branch return proved non zero length of either old or new.
        // Hence we can unwrap safely.
        let (bw_index, bw_item) = bw_eq_thru.unwrap();

        // overlapping case
        // aba
        // ababa

        // unwrap is safe due to early return if is_left or is_right
        let fw_fst_nq = fw_item.as_ref().both().unwrap();
        // unwrap is safe because if forward eq is longer then backward and is Both
        // then bw both() is Some
        let bw_fst_nq = bw_item.clone().both().unwrap();

        let mut shorter_len = None;

        if fw_index >= bw_index {
            let fw = once((fw_index, fw_item.clone())).chain(fw);

            // if eq segments are overlapping
            if fw_fst_nq.0.as_ptr() >= bw_fst_nq.0.as_ptr()
                || fw_fst_nq.1.as_ptr() >= bw_fst_nq.1.as_ptr()
            {
                // fw.map(|(_, z)| z);
                unimplemented!()
            }

            // old: "abc"
            // new: "ab-bc"

            let changed = fw.take_while(|(i, e)| {
                let mut not_greater = |a: &str, b: *const u8| match a.as_ptr().cmp(&b) {
                    Ordering::Less => true,
                    Ordering::Greater => false,
                    Ordering::Equal => {
                        if shorter_len.is_none() {
                            shorter_len = Some(i + bw_index + 2)
                        };
                        false
                    }
                };

                let not_past_back_matched = e
                    .as_ref()
                    .map_left(|a| not_greater(a, bw_fst_nq.0.as_ptr()))
                    .map_right(|b| not_greater(b, bw_fst_nq.1.as_ptr()))
                    .reduce(|a, b| a && b);

                if not_past_back_matched {
                    true
                } else {
                    // Stop when backwards equal set <= then remainder of smaller side (old/new).
                    shorter_len.map_or(true, |len| fw_index <= (len - i) + 1)
                }
            });
            let mut ph = PerfectHasher32::new(Murmur3Hasher_x86_32::default());

            let mut changed_old = Vec::with_capacity(100);
            let mut changed_new = Vec::with_capacity(100);
            for (_, e) in changed {
                e.map_left(|l| changed_old.push(ph.unique_id(l)))
                    .map_right(|r| changed_new.push(ph.unique_id(r)));
            }

            // Return diffed result
            wu_diff::diff(&changed_old, &changed_new)
        } else {
            let bw = once((bw_index, bw_item.clone())).chain(bw);

            // if eq segments are overlapping
            if bw_fst_nq.0.as_ptr() >= fw_fst_nq.0.as_ptr()
                || bw_fst_nq.1.as_ptr() >= fw_fst_nq.1.as_ptr()
            {
                // fw.map(|(_, z)| z);
                unimplemented!()
            }

            // old: "abc"
            // new: "ab-bc"

            let changed = bw.take_while(|(i, e)| {
                let mut not_greater = |a: &str, b: *const u8| match a.as_ptr().cmp(&b) {
                    Ordering::Less => true,
                    Ordering::Greater => false,
                    Ordering::Equal => {
                        if shorter_len.is_none() {
                            shorter_len = Some(i + fw_index + 2)
                        };
                        false
                    }
                };

                let not_past_back_matched = e
                    .as_ref()
                    .map_left(|a| not_greater(a, fw_fst_nq.0.as_ptr()))
                    .map_right(|b| not_greater(b, fw_fst_nq.1.as_ptr()))
                    .reduce(|a, b| a && b);

                if not_past_back_matched {
                    true
                } else {
                    // Stop when backwards equal set <= then remainder of smaller side (old/new).
                    shorter_len.map_or(true, |len| bw_index <= (len - i) + 1)
                }
            });
            let mut ph = PerfectHasher32::new(Murmur3Hasher_x86_32::default());

            let mut changed_old = Vec::with_capacity(100);
            let mut changed_new = Vec::with_capacity(100);
            for (_, e) in changed {
                e.map_left(|l| changed_old.push(ph.unique_id(l)))
                    .map_right(|r| changed_new.push(ph.unique_id(r)));
            }

            changed_new.reverse();
            changed_old.reverse();
            // Return diffed result
            wu_diff::diff(&changed_old, &changed_new)
        }
    }
}

trait FromEOB<L, R> {
    fn into_either(self) -> Option<Either<L, R>>;
}

impl<L, R> FromEOB<L, R> for EitherOrBoth<L, R> {
    fn into_either(self) -> Option<Either<L, R>> {
        match self {
            EitherOrBoth::Left(l) => Some(Either::Left(l)),
            EitherOrBoth::Right(r) => Some(Either::Right(r)),
            _ => None,
        }
    }
}
