use std::{
    fmt::{Debug, Display, Write},
    hash::{DefaultHasher, Hash, Hasher},
};

use num::Num;
use radix_fmt::radix_36;
use rand::{random, seq::SliceRandom, thread_rng};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Cuid2(u128);

impl Debug for Cuid2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cuid2(\"{}\")", radix_36(self.0))
    }
}

impl Display for Cuid2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", radix_36(self.0))
    }
}

pub fn pseudo_cuid2() -> Cuid2 {
    let process_id = std::process::id();
    let thread_id = std::thread::current().id();
    let time = std::time::SystemTime::now();
    let entropy: u64 = random();

    let mut hasher = DefaultHasher::new();
    process_id.hash(&mut hasher);
    thread_id.hash(&mut hasher);
    time.hash(&mut hasher);
    entropy.hash(&mut hasher);

    let hi = hasher.finish();
    let lo = random();
    let bytes = [hi, lo];

    let x = u128::from_ne_bytes(unsafe { std::mem::transmute(bytes) });

    let random_first_letter: char = (*"abcdefghijklmnopqrstuvwxyz"
        .as_bytes()
        .choose(&mut thread_rng())
        .unwrap())
    .into();

    let mut buffer = String::with_capacity(24);
    write!(buffer, "{}{:0>23}", random_first_letter, radix_36(x)).unwrap();
    buffer.truncate(24);

    let x: u128 = <u128 as Num>::from_str_radix(&buffer, 36).unwrap();

    Cuid2(x)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn stuff() {
        println!("{}", pseudo_cuid2());
        println!("{:?}", pseudo_cuid2());
        println!("{}", pseudo_cuid2());
        println!("{:?}", pseudo_cuid2());
        println!("{}", pseudo_cuid2());
    }
}
