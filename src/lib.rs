use std::cmp::{max, Ordering};
use std::fmt::Display;
use std::iter::once;

use either::Either;
use fasthash::murmur3::Murmur3Hasher_x86_32;
use itertools::{EitherOrBoth, EitherOrBoth::*, Itertools};
use perfect_hash::{Id, PerfectHasher32};

trait HashDiff<T, I, D> {
    fn hash_diff_vec(self, new: I) -> Vec<D>;
}

type IndexMapping<'l> = PerfectHasher32<&'l str, Murmur3Hasher_x86_32>;

pub struct HashedLines<'l> {
    index_map: IndexMapping<'l>,
    changed_old: Vec<Id<u32>>,
    changed_new: Vec<Id<u32>>,
}

#[derive(Debug, Clone)]
pub enum Change<T> {
    Equal {
        old_index: usize,
        new: T,
        new_index: usize,
        len: usize,
    },
    Delete {
        old: T,
        old_index: usize,
        len: usize,
    },
    Insert {
        old_index: usize,
        new: T,
        new_index: usize,
        new_len: usize,
    },
    Replace {
        old: T,
        old_index: usize,
        old_len: usize,
        new: T,
        new_index: usize,
        new_len: usize,
    },
}

#[derive(Debug, Clone)]
struct Changes<T> {
    diff: Vec<Change<T>>,
}

impl<T> From<Vec<Change<T>>> for Changes<T> {
    fn from(diff: Vec<Change<T>>) -> Changes<T> {
        Changes { diff }
    }
}

impl<T> Into<Vec<Change<T>>> for Changes<T> {
    fn into(self) -> Vec<Change<T>> {
        self.diff
    }
}

pub struct ChangesBuilder<'l, T>(HashedLines<'l>, Changes<Vec<T>>);

impl<'l> diffs::Diff for ChangesBuilder<'l, &'l str> {
    type Error = ();
    fn equal(&mut self, old_index: usize, new_index: usize, len: usize) -> Result<(), ()> {
        let ChangesBuilder(hashed, changes) = self;
        let new: Vec<&str> = hashed
            .index_map
            .contents(hashed.changed_new[new_index..new_index + len].iter())
            .map(|s| *s)
            .collect();
        changes.diff.push(Change::Equal {
            new,
            new_index,
            old_index,
            len,
        });
        Ok(())
    }

    fn delete(&mut self, old_index: usize, len: usize) -> Result<(), ()> {
        let ChangesBuilder(hashed, changes) = self;
        let old = hashed
            .index_map
            .contents(hashed.changed_old[old_index..old_index + len - 1].iter())
            .map(|s| *s)
            .collect();
        changes.diff.push(Change::Delete {
            old,
            old_index,
            len,
        });
        Ok(())
    }

    fn insert(&mut self, old_index: usize, new_index: usize, new_len: usize) -> Result<(), ()> {
        let ChangesBuilder(hashed, changes) = self;
        let new: Vec<&str> = hashed
            .index_map
            .contents(hashed.changed_new[new_index..new_index + new_len].iter())
            .map(|s| *s)
            .collect();
        changes.diff.push(Change::Insert {
            old_index,
            new,
            new_index,
            new_len,
        });
        Ok(())
    }

    fn replace(
        &mut self,
        old_index: usize,
        old_len: usize,
        new_index: usize,
        new_len: usize,
    ) -> Result<(), ()> {
        let ChangesBuilder(hashed, changes) = self;
        // TODO old/new should be Vec not just the first element
        let old: Vec<&str> = hashed
            .index_map
            .contents(hashed.changed_old[old_index..old_index + old_len].iter())
            .map(|s| *s)
            .collect();
        let new: Vec<&str> = hashed
            .index_map
            .contents(hashed.changed_new[new_index..new_index + new_len].iter())
            .map(|s| *s)
            .collect();
        changes.diff.push(Change::Replace {
            old,
            old_index,
            old_len,
            new,
            new_index,
            new_len,
        });
        Ok(())
    }
}

impl<'l> HashedLines<'l> {
    pub fn myers_diff_vec(self) -> Vec<Change<Vec<&'l str>>> {
        let diff = Vec::with_capacity(max(self.changed_new.len(), self.changed_old.len()));
        let unsafe_self: &Self = unsafe { &*(&self as *const Self) };
        let mut cb = ChangesBuilder(self, Changes { diff });
        let _ = diffs::myers::diff(
            &mut cb,
            &unsafe_self.changed_old,
            0,
            unsafe_self.changed_old.len(),
            &unsafe_self.changed_new,
            0,
            unsafe_self.changed_new.len(),
        );
        cb.1.diff
    }
}

trait LineDiff<'l> {
    fn lines_hash_diff(self, new: &'l str) -> Option<HashedLines>;
}

// impl<'l> Display for Diff<'l> {}

impl<'l> LineDiff<'l> for &'l str {
    fn lines_hash_diff(self, new: &'l str) -> Option<HashedLines> {
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
            return None;
        }

        let (fw_index, fw_item) = fw_eq_thru.unwrap();

        if fw_item.is_just_left() {
            // return added overall fw.next
            let mut index_map = PerfectHasher32::default();

            let mut changed_old = Vec::with_capacity(100);
            let changed_new = vec![];
            for (_, e) in fw {
                let l = e.left().unwrap();
                changed_old.push(index_map.unique_id(l))
            }

            return Some(HashedLines {
                changed_old,
                changed_new,
                index_map,
            });
        } else if fw_item.is_just_right() {
            let mut index_map = PerfectHasher32::default();

            let changed_old = vec![];
            let mut changed_new = Vec::with_capacity(100);
            for (_, e) in fw {
                let l = e.right().unwrap();
                changed_new.push(index_map.unique_id(l))
            }

            return Some(HashedLines {
                changed_old,
                changed_new,
                index_map,
            });
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
                let mut index_map = PerfectHasher32::default();

                let mut changed_old = Vec::with_capacity(100);
                let mut changed_new = Vec::with_capacity(100);
                for (_, e) in fw {
                    e.map_left(|l| changed_old.push(index_map.unique_id(l)))
                        .map_right(|r| changed_new.push(index_map.unique_id(r)));
                }
                return Some(HashedLines {
                    changed_old,
                    changed_new,
                    index_map,
                });
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

            let mut index_map = PerfectHasher32::default();

            let mut changed_old = Vec::with_capacity(100);
            let mut changed_new = Vec::with_capacity(100);
            for (_, e) in changed {
                e.map_left(|l| changed_old.push(index_map.unique_id(l)))
                    .map_right(|r| changed_new.push(index_map.unique_id(r)));
            }

            // Return diffed result
            return Some(HashedLines {
                changed_old,
                changed_new,
                index_map,
            });
        } else {
            let bw = once((bw_index, bw_item.clone())).chain(bw);

            // old: "abc"
            // new: "ab-abc"

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
            let mut index_map = PerfectHasher32::default();

            let mut changed_old = Vec::with_capacity(100);
            let mut changed_new = Vec::with_capacity(100);
            for (_, e) in changed {
                e.map_left(|l| changed_old.push(index_map.unique_id(l)))
                    .map_right(|r| changed_new.push(index_map.unique_id(r)));
            }

            changed_new.reverse();
            changed_old.reverse();
            // Return diffed result
            return Some(HashedLines {
                changed_old,
                changed_new,
                index_map,
            });
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
