use std::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

/// u128 that is represented ([std::fmt::Display] and [std::str::FromStr] impls) with [Crockford's
/// Base32](https://www.crockford.com/base32.html) without check digit
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cb32u128(u128);

const BITS: usize = u128::BITS as usize;
const DIGIT_BITS: usize = 5; // log2(32)

impl Display for Cb32u128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // heavily based on https://github.com/archer884/crockford/blob/3662a0a328a888068a368d3558cea6cde85d73c0/src/encoding.rs#L43

        // 992u128 in binary in 5 bit segments, represented in base32 as Z0
        // 000 00000 00000 00000 00000 00000 00000 00000 00000 000000 00000 11111 00000
        // \_/ \___/
        //  3    5

        const REM_BITS: usize = BITS % DIGIT_BITS;

        const REM_SHIFT: usize = BITS - REM_BITS;
        const DIGIT_SHIFT: usize = BITS - DIGIT_BITS;

        const STOP_BIT: u128 = 1 << REM_SHIFT;

        let mut x = self.0;

        if x == 0 {
            Display::fmt(&CROCKFORD_MAPPING[0], f)?;
            return Ok(());
        }

        match (x >> REM_SHIFT) as usize {
            0 => {
                x <<= REM_BITS;
                x |= 1;

                fn round_to_multiple_of_digit_bits(x: u32) -> u32 {
                    let num_multiples = x / DIGIT_BITS as u32;
                    num_multiples * DIGIT_BITS as u32
                }

                x <<= round_to_multiple_of_digit_bits(x.leading_zeros());
            }

            i => {
                x <<= REM_BITS;
                x |= 1;
                Display::fmt(&CROCKFORD_MAPPING[i], f)?;
            }
        }

        while x != STOP_BIT {
            let i = (x >> DIGIT_SHIFT) as usize;
            Display::fmt(&CROCKFORD_MAPPING[i], f)?;
            x <<= DIGIT_BITS;
        }

        Ok(())
    }
}

impl Debug for Cb32u128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cb32u128({})", self)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Cb32u128ParseError {
    InvalidDigit(char),
    UnsupportedCheckDigit(char),
}

impl FromStr for Cb32u128 {
    type Err = Cb32u128ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut res = 0u128;

        for c in s.chars() {
            if !c.is_ascii() {
                return Err(Cb32u128ParseError::InvalidDigit(c));
            }
            let digit_value = match CROCKFORD_REVERSE_MAPPING[c as usize] {
                CrmEntry::Valid(digit_value) => digit_value,
                CrmEntry::CheckDigit => return Err(Cb32u128ParseError::UnsupportedCheckDigit(c)),
                CrmEntry::Invalid => return Err(Cb32u128ParseError::InvalidDigit(c)),
            };

            res <<= DIGIT_BITS; // doesn't matter for the initial iteration
            res |= digit_value;
        }

        Ok(Cb32u128(res))
    }
}

const CROCKFORD_MAPPING: [char; 32] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J',
    'K', 'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'V', 'W', 'X', 'Y', 'Z',
];

#[derive(Copy, Clone)]
enum CrmEntry {
    Valid(u128),
    CheckDigit,
    Invalid,
}

const CROCKFORD_REVERSE_MAPPING: [CrmEntry; 256] = const {
    use CrmEntry::*;

    let mut entries = [Invalid; 256];

    macro_rules! set {
        ($($l:literal),+ => $e:expr) => {
            $(
                entries[$l as usize] = $e;
            )+
        };
    }

    set!('0', 'O', 'o' => Valid(0));
    set!('1', 'I', 'i', 'L', 'l' => Valid(1));
    set!('2' => Valid(2));
    set!('3' => Valid(3));
    set!('4' => Valid(4));
    set!('5' => Valid(5));
    set!('6' => Valid(6));
    set!('7' => Valid(7));
    set!('8' => Valid(8));
    set!('9' => Valid(9));
    set!('A', 'a' => Valid(10));
    set!('B', 'b' => Valid(11));
    set!('C', 'c' => Valid(12));
    set!('D', 'd' => Valid(13));
    set!('E', 'e' => Valid(14));
    set!('F', 'f' => Valid(15));
    set!('G', 'g' => Valid(16));
    set!('H', 'h' => Valid(17));
    set!('J', 'j' => Valid(18));
    set!('K', 'k' => Valid(19));
    set!('M', 'm' => Valid(20));
    set!('N', 'n' => Valid(21));
    set!('P', 'p' => Valid(22));
    set!('Q', 'q' => Valid(23));
    set!('R', 'r' => Valid(24));
    set!('S', 's' => Valid(25));
    set!('T', 't' => Valid(26));
    set!('V', 'v' => Valid(27));
    set!('W', 'w' => Valid(28));
    set!('X', 'x' => Valid(29));
    set!('Y', 'y' => Valid(30));
    set!('Z', 'z' => Valid(31));

    set!('*' => CheckDigit);
    set!('~' => CheckDigit);
    set!('$' => CheckDigit);
    set!('=' => CheckDigit);
    set!('U', 'u' => CheckDigit);

    entries
};

#[cfg(feature = "rand")]
mod rand_impls {
    use super::*;
    use rand::distributions::{Distribution, Standard};

    impl Distribution<Cb32u128> for Standard {
        fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Cb32u128 {
            Cb32u128(rng.gen())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::u128;

    use crate::cb32u128::{Cb32u128, Cb32u128ParseError};

    #[test]
    fn formatting_works() {
        assert_eq!(Cb32u128(0).to_string(), "0");
        assert_eq!(Cb32u128(32).to_string(), "10");
        assert_eq!(Cb32u128(0b11111_00000).to_string(), "Z0");
        assert_eq!(
            Cb32u128(u128::MAX).to_string(),
            "7ZZZZZZZZZZZZZZZZZZZZZZZZZ"
        );
    }

    #[test]
    fn parsing_works() {
        assert_eq!("0".parse(), Ok(Cb32u128(0)));
        assert_eq!("10".parse(), Ok(Cb32u128(32)));
        assert_eq!("Z0".parse(), Ok(Cb32u128(0b11111_00000)));
        assert_eq!(
            "7ZZZZZZZZZZZZZZZZZZZZZZZZZ".parse(),
            Ok(Cb32u128(u128::MAX))
        );

        assert_eq!(
            "/".parse::<Cb32u128>(),
            Err(Cb32u128ParseError::InvalidDigit('/'))
        );
        assert_eq!(
            "ą".parse::<Cb32u128>(),
            Err(Cb32u128ParseError::InvalidDigit('ą'))
        );
        assert_eq!(
            "2137💀".parse::<Cb32u128>(),
            Err(Cb32u128ParseError::InvalidDigit('💀'))
        );

        assert_eq!(
            "42069*".parse::<Cb32u128>(),
            Err(Cb32u128ParseError::UnsupportedCheckDigit('*'))
        );
    }
}
