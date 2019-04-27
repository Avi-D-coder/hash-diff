use std::cmp::Ord;
use std::cmp::{max, Ordering};
// TODO
// use std::fmt::Display;
use std::hash::Hash;
use std::iter::once;

pub use diffs;
use fasthash::murmur3::Murmur3Hasher_x86_32;
use itertools::{EitherOrBoth, EitherOrBoth::*, Itertools};
use perfect_hash::{Id, PerfectHasher32};

type IndexMapping<T> = PerfectHasher32<T, Murmur3Hasher_x86_32>;

pub struct Hashed<T> {
    pub index_map: IndexMapping<T>,
    pub changed_old: Vec<Id<u32>>,
    pub changed_new: Vec<Id<u32>>,
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
    pub diff: Vec<Change<T>>,
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

pub struct Segment<T> {
    pub index: usize,
    pub seg: T,
}

pub struct ChangesBuilder<T>(Hashed<T>, Changes<Vec<T>>);

impl<'l, T> diffs::Diff for ChangesBuilder<&'l T>
where
    T: Ord + Hash,
{
    type Error = ();
    fn equal(&mut self, old_index: usize, new_index: usize, len: usize) -> Result<(), ()> {
        let ChangesBuilder(hashed, changes) = self;
        let new = hashed
            .index_map
            .contents(hashed.changed_new[new_index..new_index + len].iter())
            .cloned()
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
            .cloned()
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
        let new = hashed
            .index_map
            .contents(hashed.changed_new[new_index..new_index + new_len].iter())
            .cloned()
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
        let old = hashed
            .index_map
            .contents(hashed.changed_old[old_index..old_index + old_len].iter())
            .cloned()
            .collect();
        let new = hashed
            .index_map
            .contents(hashed.changed_new[new_index..new_index + new_len].iter())
            .cloned()
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

impl<'l, T> Hashed<&'l T>
where
    T: Ord + Hash,
{
    pub fn myers_diff_vec(self) -> Vec<Change<Vec<&'l T>>> {
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

pub trait ContentPosition: Clone {
    type Content: Ord + Hash + Clone;
    fn content(&self) -> Self::Content;

    type Position: Ord;
    fn pos(&self) -> Self::Position;
}

pub trait Segments {
    type Iter: DoubleEndedIterator<Item = Self::Segment> + Clone;
    type Segment: ContentPosition;
    fn segments(self) -> Self::Iter;
}

pub trait Position {}

impl<'l> Segments for &'l str {
    type Iter = std::str::Lines<'l>;
    type Segment = &'l str;
    fn segments(self) -> std::str::Lines<'l> {
        self.lines()
    }
}
impl<'l> ContentPosition for &'l str {
    type Content = Self;
    fn content(&self) -> Self {
        self
    }

    type Position = *const u8;
    fn pos(&self) -> *const u8 {
        self.as_ptr()
    }
}

pub trait HashChanged<I, C> {
    type Item;
    fn hash_changed(self, new: I) -> Option<Hashed<C>>;
}

// impl<'l> Display for Diff<'l> {}

impl<O, N, S> HashChanged<N, S::Content> for O
where
    O: Segments<Segment = S>,
    N: Segments<Segment = S>,
    S: ContentPosition,
{
    type Item = S;
    fn hash_changed(self, new: N) -> Option<Hashed<S::Content>> {
        let old = self.segments();
        let new = new.segments();

        let (fw_eq_thru, fw) = {
            let mut fw = old.clone().zip_longest(new.clone()).enumerate();
            let mut pre: Option<(usize, EitherOrBoth<S, S>)> = None;

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
                        pre = Some((i, Both(o.clone(), n.clone())));

                        if o.content() == n.content() {
                            continue;
                        } else {
                            break (pre, fw);
                        }
                    }
                    m => break (m, fw),
                }
            }
        };

        let (fw_index, fw_item) = fw_eq_thru?;

        if fw_item.is_just_left() {
            // return added overall fw.next
            let mut index_map = PerfectHasher32::default();

            let mut changed_old = Vec::with_capacity(100);
            let changed_new = vec![];
            for (_, e) in fw {
                let l = e.left().unwrap();
                changed_old.push(index_map.unique_id(l.content()))
            }

            return Some(Hashed {
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
                changed_new.push(index_map.unique_id(l.content()))
            }

            return Some(Hashed {
                changed_old,
                changed_new,
                index_map,
            });
        }

        let (bw_eq_thru, bw) = {
            let mut bw = old.rev().zip_longest(new.rev()).enumerate();
            let mut pre: Option<(usize, EitherOrBoth<S, S>)> = None;

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
                        pre = Some((i, Both(o.clone(), n.clone())));

                        if o.content() == n.content() {
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
            if fw_fst_nq.0.pos() >= bw_fst_nq.0.pos() || fw_fst_nq.1.pos() >= bw_fst_nq.1.pos() {
                let mut index_map = PerfectHasher32::default();

                let mut changed_old = Vec::with_capacity(100);
                let mut changed_new = Vec::with_capacity(100);
                for (_, e) in fw {
                    e.map_left(|l| changed_old.push(index_map.unique_id(l.content())))
                        .map_right(|r| changed_new.push(index_map.unique_id(r.content())));
                }
                return Some(Hashed {
                    changed_old,
                    changed_new,
                    index_map,
                });
            }

            // old: "abc"
            // new: "ab-bc"

            let changed = fw.take_while(|(i, e)| {
                let mut not_greater = |a: &S, b: S::Position| {
                    let a_pos: S::Position = a.pos();
                    match a_pos.cmp(&b) {
                        Ordering::Less => true,
                        Ordering::Greater => false,
                        Ordering::Equal => {
                            if shorter_len.is_none() {
                                shorter_len = Some(i + bw_index + 2)
                            };
                            false
                        }
                    }
                };

                let not_past_back_matched = e
                    .as_ref()
                    .map_left(|a| not_greater(a, bw_fst_nq.0.pos()))
                    .map_right(|b| not_greater(b, bw_fst_nq.1.pos()))
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
                e.map_left(|l| changed_old.push(index_map.unique_id(l.content())))
                    .map_right(|r| changed_new.push(index_map.unique_id(r.content())));
            }

            // Return diffed result
            Some(Hashed {
                changed_old,
                changed_new,
                index_map,
            })
        } else {
            let bw = once((bw_index, bw_item.clone())).chain(bw);

            // old: "abc"
            // new: "ab-abc"

            let changed = bw.take_while(|(i, e)| {
                let mut not_greater = |a: &S, b: S::Position| {
                    let a_pos: S::Position = a.pos();
                    match a_pos.cmp(&b) {
                        Ordering::Less => true,
                        Ordering::Greater => false,
                        Ordering::Equal => {
                            if shorter_len.is_none() {
                                shorter_len = Some(i + fw_index + 2)
                            };
                            false
                        }
                    }
                };

                let not_past_back_matched = e
                    .as_ref()
                    .map_left(|a| not_greater(a, fw_fst_nq.0.pos()))
                    .map_right(|b| not_greater(b, fw_fst_nq.1.pos()))
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
                e.map_left(|l| changed_old.push(index_map.unique_id(l.content())))
                    .map_right(|r| changed_new.push(index_map.unique_id(r.content())));
            }

            changed_new.reverse();
            changed_old.reverse();
            // Return diffed result
            Some(Hashed {
                changed_old,
                changed_new,
                index_map,
            })
        }
    }
}
