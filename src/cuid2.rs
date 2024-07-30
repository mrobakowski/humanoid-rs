use std::{
    fmt::{Debug, Display, Write}, hash::{DefaultHasher, Hash, Hasher}, ops::Deref, str::FromStr
};

use num::Num;
use radix_fmt::radix_36;
use rand::{random, seq::SliceRandom, thread_rng, Rng};
use thiserror::Error;

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

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Cuid2ParseError {
    #[error("wrong length (required 24)")]
    WrongLength,
    #[error("illegal character")]
    IllegalCharacter,
}

impl FromStr for Cuid2 {
    type Err = Cuid2ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 24 {
            return Err(Cuid2ParseError::WrongLength);
        }
        let encoded = u128::from_str_radix(s, 36).map_err(|_| Cuid2ParseError::IllegalCharacter)?;
        Ok(Cuid2(encoded))
    }
}

pub fn pseudo_cuid2() -> Cuid2 {
    pseudo_cuid2_from_rng(&mut thread_rng())
}

pub fn pseudo_cuid2_from_rng(rng: &mut (impl Rng + ?Sized)) -> Cuid2 {
    let process_id = std::process::id();
    let thread_id = std::thread::current().id();
    let time = std::time::SystemTime::now();
    let entropy: u64 = rng.gen();

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

    let x: u128 = u128::from_str_radix(&buffer, 36).unwrap();

    Cuid2(x)
}

#[cfg(feature = "rand")]
mod rand_impls {
    use rand::distributions::{Distribution, Standard};

    use super::{pseudo_cuid2_from_rng, Cuid2};

    impl Distribution<Cuid2> for Standard {
        fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Cuid2 {
            pseudo_cuid2_from_rng(rng)
        }
    }
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
