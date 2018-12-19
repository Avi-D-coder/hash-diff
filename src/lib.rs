use std::hash::BuildHasher;
use std::iter::DoubleEndedIterator;

use fasthash::murmur3::Murmur3_x86_32;
use perfect_hash::PerfectHasher32;
use wu_diff::*;

trait HashDiff<T, I, D> {
    fn hash_diff_vec(self, new: I) -> Vec<D>;
}


// impl<T, I> HashDiff<T, I, DiffResult> for I
// where
//     I: DoubleEndedIterator<Item = T>,
//     T: PartialOrd,
// {
//     fn hash_diff(mut self: Self, mut new: I) -> Vec<DiffResult> {
        // fn eq<I, T, F>(old: &mut I, new: &mut I, next: F) -> ((Option<T>, Option<T>), usize)
        // where
        //     T: PartialEq,
        //     F: Fn(&mut I) -> Option<T>,
        // {
        //     let mut count = 0;
        //     loop {
        //         let o = next(old);
        //         let n = next(new);
        //         if o.is_some() && n.is_some() && o == n {
        //             count += 1;
        //         } else {
        //             return ((o, n), count);
        //         }
        //     }
        // }

        // let next = |i: &mut I| i.next();
        // let next_back = |i: &mut I| i.next_back();

        // let (first_ne, eq_to) = eq(&mut self, &mut new, next);

        // if eq_to == 0 && first_ne.0.is_none() && first_ne.1.is_none() {
        //     return vec![];
        // }

        // let (back_first_ne, back_eq_to) = eq(&mut self, &mut new, next_back);

        // // abc
        // // ababc
        // let back_eq_to = match back_first_ne {
        //     // old is shorter than new
        //     (None, Some(n)) => {
        //         if first_ne.0.unwrap() == n {
        //             back_eq_to + 1
        //         } else {
        //             back_eq_to
        //         }
        //     }
        //     (Some(o), None) => {
        //         if first_ne.0.unwrap() == o {
        //             back_eq_to + 1
        //         } else {
        //             back_eq_to
        //         }
        //     }
        //     (Some(_), Some(_)) => back_eq_to,
        //     (None, None) => return vec![], // no change
        // };

        // let eq_to = self
        //     .zip(new)
        //     .take_while(|(o, n)| o == n)
        //     .enumerate()
        //     .last()
        //     .map_or(0, |(eq_thru, _)| eq_thru + 1);

        // let eq_back_to = self
        //     .rev()
        //     .zip(new.rev())
        //     .enumerate()
        //     .take_while(|(i, (o, n))| o == n && *i != eq_to - 1 [> FIXME <])
        //     .last()
        //     .map_or(0, |(eq_thru, _)| eq_thru - 1);

        // let perfect_hash = PerfectHasher32::with_capacity_and_hasher(
        //     (eq_to + eq_back_to + 1000) as u32,
        //     Murmur3_x86_32 {}.build_hasher(),
        // );

//         unimplemented!()
//     }
// }

// #[test]
// fn diff_test() {
//     &"".lines().diff("".lines());
// }
