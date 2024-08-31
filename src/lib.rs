use std::{
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
    str::{self, FromStr},
};

use thiserror::Error;

#[cfg(feature = "cuid2")]
pub mod cuid2;

#[cfg(feature = "cb32u128")]
pub mod cb32u128;

pub trait Prefix {
    const VALUE: &str;
}

/// Implementation of [Prefix] for short (len < 15) strings. Refer to this type via the [prefix]
/// macro, which encodes the given `&str` into a u128.
///
/// This is a hack around the lack of const &str generic parameters.
///
/// # Example
/// ```rust
/// use humanoid::{prefix, Prefix};
/// type CustomerPrefix = prefix!("cus");
///
/// assert_eq!(<CustomerPrefix as Prefix>::VALUE, "cus");
/// ```
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct ShortPrefix<const BYTES: u128>;

#[doc(hidden)]
pub const fn encode_bytes(s: &str) -> u128 {
    let len = s.len();
    assert!(len <= 15, "prefix string too long; max length is 15");
    let len = len as u8;
    let mut bytes = [0; 16];
    bytes[0] = len;
    let len = len as usize;
    let mut i = 0;
    while i < len {
        bytes[i + 1] = s.as_bytes()[i];
        i += 1;
    }

    u128::from_ne_bytes(bytes)
}

const fn decode_bytes(bytes: &u128) -> &str {
    // SAFETY: u128 has 16 bytes, and a reference to it is probably very aligned
    //  all bit patterns are also valid for u128, so this transmute is legit
    let bytes: &[u8; 16] = unsafe { std::mem::transmute(bytes) };
    let len = bytes[0] as usize;
    assert!(len <= 15);
    // SAFETY: this effectively does &bytes[1..len+1], but that's not allowed in const rust yet
    //  there are 16 bytes in u128, len is at most 15, so we're always in a valid range
    let bytes = unsafe { std::slice::from_raw_parts((bytes as *const u8).offset(1), len) };

    match str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => panic!("invalid input"),
    }
}

impl<const B: u128> Debug for ShortPrefix<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ShortPrefix<{:?}>", decode_bytes(&B))
    }
}

impl<const B: u128> Display for ShortPrefix<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", decode_bytes(&B))
    }
}

impl<const B: u128> Hash for ShortPrefix<B> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Self::VALUE.hash(state)
    }
}

impl<const B: u128> Prefix for ShortPrefix<B> {
    const VALUE: &str = decode_bytes(&B);
}

#[macro_export]
macro_rules! prefix {
    ($s:literal) => {
        $crate::ShortPrefix<{$crate::encode_bytes($s)}>
    };
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct PrefixedId<P: Prefix, T>(pub T, PhantomData<P>);

impl<P: Prefix, T> PrefixedId<P, T> {
    pub fn from_str_required_prefix(
        s: &str,
    ) -> Result<PrefixedId<P, T>, PrefixedIdParseError<T::Err>>
    where
        T: FromStr,
    {
        let s = s
            .strip_prefix(P::VALUE)
            .ok_or_else(|| PrefixedIdParseError::NoPrefix(P::VALUE))?;
        let s = s
            .strip_prefix('_')
            .ok_or_else(|| PrefixedIdParseError::NoUnderscore)?;
        let t = T::from_str(s)?;

        Ok(PrefixedId(t, Default::default()))
    }

    pub fn from_str_optional_prefix(s: &str) -> Result<PrefixedId<P, T>, T::Err>
    where
        T: FromStr,
    {
        let s = s.strip_prefix(P::VALUE).unwrap_or(s);
        let s = s.strip_prefix('_').unwrap_or(s);
        let t = T::from_str(s)?;

        Ok(PrefixedId(t, Default::default()))
    }

    pub fn from_id(id: T) -> PrefixedId<P, T> {
        PrefixedId(id, Default::default())
    }
}

#[macro_export]
macro_rules! PrefixedId {
    ($s:literal, $t:ty) => {
        $crate::PrefixedId<$crate::prefix!($s), $t>
    };
}

impl<P: Prefix, T: Display> Display for PrefixedId<P, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", P::VALUE, self.0)
    }
}

// TODO: T: Display is not ideal, but T: Debug would double-quote strings
impl<P: Prefix, T: Display> Debug for PrefixedId<P, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PrefixedId(\"{}_{}\")", P::VALUE, self.0)
    }
}

impl<P, T> Hash for PrefixedId<P, T>
where
    P: Prefix + Hash,
    T: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        P::VALUE.hash(state);
        self.0.hash(state);
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum PrefixedIdParseError<T> {
    #[error("Missing {0:?} prefix.")]
    NoPrefix(&'static str),
    #[error("Missing '_' separating the prefix from the id part.")]
    NoUnderscore,
    #[error("Couldn't parse the id.")]
    Other(#[from] T),
}

impl<P: Prefix, T: FromStr> FromStr for PrefixedId<P, T> {
    type Err = PrefixedIdParseError<T::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        PrefixedId::from_str_required_prefix(s)
    }
}

#[cfg(feature = "rand")]
mod rand_impls {
    use super::*;
    use rand::distributions::{Distribution, Standard};

    impl<P: Prefix, T> Distribution<PrefixedId<P, T>> for Standard
    where
        Standard: Distribution<T>,
    {
        fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> PrefixedId<P, T> {
            PrefixedId::from_id(self.sample(rng))
        }
    }
}

#[cfg(feature = "serde")]
mod serde_impls {
    use super::*;
    use serde::{de::Visitor, Deserialize, Serialize, Serializer};

    impl<P: Prefix, T: Display> Serialize for PrefixedId<P, T> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.collect_str(self)
        }
    }

    impl<'de, P: Prefix, T: FromStr> Deserialize<'de> for PrefixedId<P, T> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct V<P, T>(PhantomData<(P, T)>);
            impl<'de, P: Prefix, T: FromStr> Visitor<'de> for V<P, T> {
                type Value = PrefixedId<P, T>;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    write!(formatter, "an id prefixed with {}", P::VALUE)
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Self::Value::from_str(v).map_err(|e| E::custom(e))
                }
            }

            deserializer.deserialize_str(V(Default::default()))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    type CustomerId = PrefixedId!("cus", String);

    #[test]
    fn display() {
        let cid = CustomerId::from_id("1234".into());
        assert_eq!(cid.to_string(), "cus_1234")
    }

    #[test]
    fn parse_required() {
        let cid = CustomerId::from_str("cus_1234");
        assert_eq!(cid, Ok(CustomerId::from_id("1234".into())))
    }

    #[test]
    fn parse_optional_some() {
        let cid = CustomerId::from_str_optional_prefix("cus_1234");
        assert_eq!(cid, Ok(CustomerId::from_id("1234".into())))
    }

    #[test]
    fn parse_optional_none() {
        let cid = CustomerId::from_str_optional_prefix("1234");
        assert_eq!(cid, Ok(CustomerId::from_id("1234".into())))
    }

    #[cfg(feature = "cuid2")]
    #[test]
    fn parse_prefixed_cuid() {
        type CustomerId = PrefixedId!("cus", cuid2::Cuid2);
        let cid = CustomerId::from_str_required_prefix("cus_123456789012345678901234");
        assert_eq!(
            cid,
            Ok(CustomerId::from_id(
                "123456789012345678901234".parse().unwrap()
            ))
        )
    }

    #[cfg(feature = "cuid2")]
    #[test]
    fn generate_prefixed_cuid() {
        use rand::random;
        type CustomerId = PrefixedId!("cus", cuid2::Cuid2);
        let cid: CustomerId = random();
        assert!(cid.to_string().starts_with("cus_"));
        println!("Debug: {:?}", cid);
        println!("Display: {}", cid);
    }

    #[cfg(feature = "cb32u128")]
    #[test]
    fn parse_prefixed_cb32u128() {
        type CustomerId = PrefixedId!("cus", cb32u128::Cb32u128);
        let cid = CustomerId::from_str_required_prefix("cus_2137PAPA");
        assert_eq!(cid, Ok(CustomerId::from_id("2137PAPA".parse().unwrap())))
    }

    #[cfg(all(feature = "cb32u128", feature = "rand"))]
    #[test]
    fn generate_prefixed_cb32u128() {
        use rand::random;
        type CustomerId = PrefixedId!("cus", cb32u128::Cb32u128);
        let cid: CustomerId = random();
        assert!(cid.to_string().starts_with("cus_"));
        println!("Debug: {:?}", cid);
        println!("Display: {}", cid);
    }
}
