//! Marks snapshot fields that a compact network encoding must keep bit-exact.
//!
//! Use as `#[serde(with = "crate::exact")]`. The value is wrapped in a newtype
//! named [`NAME`]; ordinary formats (bincode, JSON) treat that newtype as
//! transparent, so their output is unchanged. A lossy wire codec can recognise
//! the name and skip float rounding for everything inside it. Race time, lap
//! timing and the AI plan inputs live here: rounding them would either drift
//! the displayed timing or force AI drivers to rebuild their lap plans.
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::marker::PhantomData;

/// Newtype name recognised by precision-reducing encoders.
pub const NAME: &str = "racecraft::exact";

pub fn serialize<T: Serialize, S: Serializer>(value: &T, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_newtype_struct(NAME, value)
}

pub fn deserialize<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<T, D::Error> {
    struct Exact<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>> de::Visitor<'de> for Exact<T> {
        type Value = T;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("an exact value")
        }
        fn visit_newtype_struct<D: Deserializer<'de>>(self, d: D) -> Result<T, D::Error> {
            T::deserialize(d)
        }
    }
    deserializer.deserialize_newtype_struct(NAME, Exact(PhantomData))
}
