use std::hash::BuildHasher;
use std::iter::DoubleEndedIterator;

use fasthash::murmur3::Murmur3_x86_32;
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

        let mut old_len = 0;
        let mut new_len = 0;

        let (fw_eq_thru, fw) = {
            let mut fw = old.clone().zip(new.clone()).enumerate();
            let mut pre: Option<(usize, (&str, &str))> = None;

            loop {
                match fw.next() {
                    None => {
                        if pre.is_some() {
                            break (pre, fw);
                        } else {
                            break (None, fw);
                        }
                    }
                    Some((i, (o, n))) => {
                        if o == n {
                            pre = Some((i, (o, n)));
                            continue;
                        } else {
                            break (pre, fw);
                        }
                    }
                }
            }
        };

        let (bw_eq_thru, bw) = {
            let mut bw = old.rev().zip(new.rev()).enumerate();
            let mut pre: Option<(usize, (&str, &str))> = None;

            loop {
                match bw.next() {
                    None => {
                        if pre.is_some() {
                            break (pre, bw);
                        } else {
                            break (None, bw);
                        }
                    }
                    Some((i, (o, n))) => {
                        if o == n {
                            pre = Some((i, (o, n)));
                            continue;
                        } else {
                            break (pre, bw);
                        }
                    }
                }
            }
        };

        // TODO use ziplongest from itertools
        if fw_eq_thru >= bw_eq_thru {
            // fw.take_while(|s| )
            unimplemented!()
        } else {
            unimplemented!()
        }
    }
}
