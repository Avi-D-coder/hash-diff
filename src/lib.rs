use either::Either;
use fasthash::murmur3::Murmur3_x86_32;
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

        let fw_eq_thru = fw_eq_thru.unwrap();

        if fw_eq_thru.1.is_left() || fw_eq_thru.1.is_right() {
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
        let bw_eq_thru = bw_eq_thru.unwrap();

        if fw_eq_thru.0 >= bw_eq_thru.0 {
            unimplemented!()
        } else {
            unimplemented!()
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
