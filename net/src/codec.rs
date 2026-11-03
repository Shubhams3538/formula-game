//! Compact snapshot encoding: floats as `f32` in their own stream, everything else exact.
//!
//! Race state is dominated by `f64` values (about 1,400 of them for 16 cars).
//! Sending them as `f32` halves the payload, and keeping them in a separate,
//! fixed-width stream lets an XOR against an older snapshot zero out every
//! value that did not change, which then compresses away. Fields wrapped by
//! [`racecraft_sim::exact`] (race time, lap timing, AI plan inputs) stay
//! bit-exact. The format is not self-describing; both sides decode with the
//! same Rust types, exactly like bincode.
//!
//! The server applies the decoded (rounded) state to its own race after every
//! snapshot, so client and server continue from bit-identical state.
use serde::{de, ser, Deserialize, Serialize};
use std::fmt;

/// One encoded value split into an exact stream and a rounded float stream.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Encoded {
    pub bytes: Vec<u8>,
    pub floats: Vec<u8>,
}
impl Encoded {
    /// XOR each stream against `base`. Unchanged values become zero bytes.
    /// Applying the same function to the result with the same base restores it.
    pub fn xor(&self, base: &Encoded) -> Encoded {
        fn xor(a: &[u8], b: &[u8]) -> Vec<u8> {
            a.iter()
                .enumerate()
                .map(|(i, x)| x ^ b.get(i).copied().unwrap_or(0))
                .collect()
        }
        Encoded {
            bytes: xor(&self.bytes, &base.bytes),
            floats: xor(&self.floats, &base.floats),
        }
    }
    pub fn len(&self) -> usize {
        self.bytes.len() + self.floats.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug)]
pub struct Error(String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}
impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}
type Result<T> = std::result::Result<T, Error>;

pub fn encode<T: Serialize>(value: &T) -> std::result::Result<Encoded, String> {
    let mut s = Serializer {
        out: Encoded::default(),
        exact: 0,
    };
    value.serialize(&mut s).map_err(|e| e.to_string())?;
    Ok(s.out)
}
pub fn decode<'a, T: Deserialize<'a>>(encoded: &'a Encoded) -> std::result::Result<T, String> {
    let mut d = Deserializer {
        bytes: &encoded.bytes,
        floats: &encoded.floats,
        exact: 0,
    };
    let value = T::deserialize(&mut d).map_err(|e| e.to_string())?;
    if !d.bytes.is_empty() || !d.floats.is_empty() {
        return Err("trailing snapshot data".into());
    }
    Ok(value)
}

struct Serializer {
    out: Encoded,
    exact: u32,
}
impl Serializer {
    fn put(&mut self, b: &[u8]) {
        self.out.bytes.extend_from_slice(b);
    }
    fn len(&mut self, len: usize) -> Result<()> {
        let len = u32::try_from(len).map_err(|_| Error("sequence too long".into()))?;
        self.put(&len.to_le_bytes());
        Ok(())
    }
}
impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.put(&[v as u8]);
        Ok(())
    }
    fn serialize_i8(self, v: i8) -> Result<()> {
        self.put(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_i16(self, v: i16) -> Result<()> {
        self.put(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_i32(self, v: i32) -> Result<()> {
        self.put(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_i64(self, v: i64) -> Result<()> {
        self.put(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_u8(self, v: u8) -> Result<()> {
        self.put(&[v]);
        Ok(())
    }
    fn serialize_u16(self, v: u16) -> Result<()> {
        self.put(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_u32(self, v: u32) -> Result<()> {
        self.put(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_u64(self, v: u64) -> Result<()> {
        self.put(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_f32(self, v: f32) -> Result<()> {
        self.out.floats.extend_from_slice(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_f64(self, v: f64) -> Result<()> {
        if self.exact > 0 {
            self.put(&v.to_le_bytes());
        } else {
            self.out.floats.extend_from_slice(&(v as f32).to_le_bytes());
        }
        Ok(())
    }
    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_u32(v as u32)
    }
    fn serialize_str(self, v: &str) -> Result<()> {
        self.serialize_bytes(v.as_bytes())
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.len(v.len())?;
        self.put(v);
        Ok(())
    }
    fn serialize_none(self) -> Result<()> {
        self.put(&[0]);
        Ok(())
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<()> {
        self.put(&[1]);
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<()> {
        Ok(())
    }
    fn serialize_unit_variant(self, _: &'static str, index: u32, _: &'static str) -> Result<()> {
        self.serialize_u32(index)
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<()> {
        let exact = name == racecraft_sim::exact::NAME;
        self.exact += exact as u32;
        let result = value.serialize(&mut *self);
        self.exact -= exact as u32;
        result
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        index: u32,
        _: &'static str,
        value: &T,
    ) -> Result<()> {
        self.serialize_u32(index)?;
        value.serialize(self)
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self> {
        self.len(len.ok_or(Error("sequence length required".into()))?)?;
        Ok(self)
    }
    fn serialize_tuple(self, _: usize) -> Result<Self> {
        Ok(self)
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self> {
        Ok(self)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        index: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self> {
        self.serialize_u32(index)?;
        Ok(self)
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self> {
        self.len(len.ok_or(Error("map length required".into()))?)?;
        Ok(self)
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self> {
        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        index: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self> {
        self.serialize_u32(index)?;
        Ok(self)
    }
}
macro_rules! compound {
    ($($trait:ident :: $method:ident),*) => {$(
        impl<'a> ser::$trait for &'a mut Serializer {
            type Ok = ();
            type Error = Error;
            fn $method<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
                value.serialize(&mut **self)
            }
            fn end(self) -> Result<()> {
                Ok(())
            }
        }
    )*};
}
compound!(
    SerializeSeq::serialize_element,
    SerializeTuple::serialize_element,
    SerializeTupleStruct::serialize_field,
    SerializeTupleVariant::serialize_field
);
impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<()> {
        key.serialize(&mut **self)
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        Ok(())
    }
}
impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, _: &'static str, v: &T) -> Result<()> {
        v.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        Ok(())
    }
}
impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, _: &'static str, v: &T) -> Result<()> {
        v.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        Ok(())
    }
}

struct Deserializer<'a> {
    bytes: &'a [u8],
    floats: &'a [u8],
    exact: u32,
}
impl<'a> Deserializer<'a> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N]> {
        if self.bytes.len() < N {
            return Err(Error("truncated snapshot".into()));
        }
        let (head, rest) = self.bytes.split_at(N);
        self.bytes = rest;
        Ok(head.try_into().unwrap())
    }
    fn float(&mut self) -> Result<f32> {
        if self.floats.len() < 4 {
            return Err(Error("truncated snapshot floats".into()));
        }
        let (head, rest) = self.floats.split_at(4);
        self.floats = rest;
        Ok(f32::from_le_bytes(head.try_into().unwrap()))
    }
    fn len(&mut self) -> Result<usize> {
        let len = u32::from_le_bytes(self.take()?) as usize;
        // Every element needs at least one byte or one float, so a length past
        // the remaining input is malformed. This bounds allocations.
        if len > self.bytes.len() + self.floats.len() {
            return Err(Error("invalid sequence length".into()));
        }
        Ok(len)
    }
}
struct Access<'a, 'b> {
    de: &'b mut Deserializer<'a>,
    left: usize,
}
impl<'de, 'b> de::SeqAccess<'de> for Access<'de, 'b> {
    type Error = Error;
    fn next_element_seed<T: de::DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        if self.left == 0 {
            return Ok(None);
        }
        self.left -= 1;
        seed.deserialize(&mut *self.de).map(Some)
    }
    fn size_hint(&self) -> Option<usize> {
        Some(self.left)
    }
}
impl<'de, 'b> de::MapAccess<'de> for Access<'de, 'b> {
    type Error = Error;
    fn next_key_seed<K: de::DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.left == 0 {
            return Ok(None);
        }
        self.left -= 1;
        seed.deserialize(&mut *self.de).map(Some)
    }
    fn next_value_seed<V: de::DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        seed.deserialize(&mut *self.de)
    }
}
impl<'de, 'b> de::EnumAccess<'de> for &'b mut Deserializer<'de> {
    type Error = Error;
    type Variant = Self;
    fn variant_seed<V: de::DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self)> {
        let index = u32::from_le_bytes(self.take()?);
        let value = seed.deserialize(de::IntoDeserializer::<Error>::into_deserializer(index))?;
        Ok((value, self))
    }
}
impl<'de, 'b> de::VariantAccess<'de> for &'b mut Deserializer<'de> {
    type Error = Error;
    fn unit_variant(self) -> Result<()> {
        Ok(())
    }
    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        seed.deserialize(self)
    }
    fn tuple_variant<V: de::Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        visitor.visit_seq(Access { de: self, left: len })
    }
    fn struct_variant<V: de::Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_seq(Access {
            de: self,
            left: fields.len(),
        })
    }
}
macro_rules! number {
    ($($method:ident => $visit:ident($ty:ty)),*) => {$(
        fn $method<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
            visitor.$visit(<$ty>::from_le_bytes(self.take()?))
        }
    )*};
}
impl<'de, 'b> de::Deserializer<'de> for &'b mut Deserializer<'de> {
    type Error = Error;
    fn deserialize_any<V: de::Visitor<'de>>(self, _: V) -> Result<V::Value> {
        Err(Error("snapshot format is not self-describing".into()))
    }
    fn deserialize_ignored_any<V: de::Visitor<'de>>(self, _: V) -> Result<V::Value> {
        Err(Error("snapshot format is not self-describing".into()))
    }
    fn deserialize_bool<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.take::<1>()?[0] {
            0 => visitor.visit_bool(false),
            1 => visitor.visit_bool(true),
            _ => Err(Error("invalid bool".into())),
        }
    }
    number!(
        deserialize_i8 => visit_i8(i8),
        deserialize_i16 => visit_i16(i16),
        deserialize_i32 => visit_i32(i32),
        deserialize_i64 => visit_i64(i64),
        deserialize_u8 => visit_u8(u8),
        deserialize_u16 => visit_u16(u16),
        deserialize_u32 => visit_u32(u32),
        deserialize_u64 => visit_u64(u64)
    );
    fn deserialize_f32<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_f32(self.float()?)
    }
    fn deserialize_f64<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.exact > 0 {
            visitor.visit_f64(f64::from_le_bytes(self.take()?))
        } else {
            visitor.visit_f64(self.float()? as f64)
        }
    }
    fn deserialize_char<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let code = u32::from_le_bytes(self.take()?);
        visitor.visit_char(char::from_u32(code).ok_or(Error("invalid char".into()))?)
    }
    fn deserialize_str<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.len()?;
        if self.bytes.len() < len {
            return Err(Error("truncated string".into()));
        }
        let (text, rest) = self.bytes.split_at(len);
        self.bytes = rest;
        visitor.visit_borrowed_str(std::str::from_utf8(text).map_err(|e| Error(e.to_string()))?)
    }
    fn deserialize_string<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }
    fn deserialize_bytes<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.len()?;
        if self.bytes.len() < len {
            return Err(Error("truncated bytes".into()));
        }
        let (bytes, rest) = self.bytes.split_at(len);
        self.bytes = rest;
        visitor.visit_borrowed_bytes(bytes)
    }
    fn deserialize_byte_buf<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_bytes(visitor)
    }
    fn deserialize_option<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.take::<1>()?[0] {
            0 => visitor.visit_none(),
            1 => visitor.visit_some(self),
            _ => Err(Error("invalid option tag".into())),
        }
    }
    fn deserialize_unit<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_unit()
    }
    fn deserialize_unit_struct<V: de::Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_unit()
    }
    fn deserialize_newtype_struct<V: de::Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        let exact = name == racecraft_sim::exact::NAME;
        self.exact += exact as u32;
        let result = visitor.visit_newtype_struct(&mut *self);
        self.exact -= exact as u32;
        result
    }
    fn deserialize_seq<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let left = self.len()?;
        visitor.visit_seq(Access { de: self, left })
    }
    fn deserialize_tuple<V: de::Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        visitor.visit_seq(Access { de: self, left: len })
    }
    fn deserialize_tuple_struct<V: de::Visitor<'de>>(
        self,
        _: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_seq(Access { de: self, left: len })
    }
    fn deserialize_map<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let left = self.len()?;
        visitor.visit_map(Access { de: self, left })
    }
    fn deserialize_struct<V: de::Visitor<'de>>(
        self,
        _: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_seq(Access {
            de: self,
            left: fields.len(),
        })
    }
    fn deserialize_enum<V: de::Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_enum(self)
    }
    fn deserialize_identifier<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_u32(visitor)
    }
    fn is_human_readable(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Kind {
        A,
        B(f64),
        C { x: u8, y: Option<f64> },
    }
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Sample {
        tick: u64,
        speed: f64,
        #[serde(with = "racecraft_sim::exact")]
        time: f64,
        kinds: Vec<Kind>,
        name: String,
        pair: (bool, i16),
    }

    #[test]
    fn floats_round_to_f32_but_exact_fields_and_structure_survive() {
        let value = Sample {
            tick: 9,
            speed: 0.1,
            time: 1234.000_000_1,
            kinds: vec![Kind::A, Kind::B(2.5), Kind::C { x: 3, y: Some(0.3) }],
            name: "spa".into(),
            pair: (true, -7),
        };
        let encoded = encode(&value).unwrap();
        let back: Sample = decode(&encoded).unwrap();
        assert_eq!(back.speed, 0.1f32 as f64);
        assert_eq!(back.time, 1234.000_000_1);
        assert_eq!(back.kinds[2], Kind::C { x: 3, y: Some(0.3f32 as f64) });
        assert_eq!((back.tick, back.name.as_str(), back.pair), (9, "spa", (true, -7)));
        // Rounded values are stable: a second round trip is bit-identical.
        assert_eq!(encode(&back).unwrap(), encoded);
    }

    #[test]
    fn xor_delta_restores_the_original_and_zeroes_unchanged_values() {
        let a = encode(&vec![1.0f64, 2.0, 3.0]).unwrap();
        let b = encode(&vec![1.0f64, 2.5, 3.0, 4.0]).unwrap();
        let delta = b.xor(&a);
        assert_eq!(&delta.floats[..4], &[0; 4]);
        assert_eq!(delta.xor(&a), b);
    }

    #[test]
    fn malformed_input_is_rejected_without_panicking() {
        let good = encode(&vec![1u64, 2, 3]).unwrap();
        let mut long = good.clone();
        long.bytes[..4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode::<Vec<u64>>(&long).is_err());
        let mut short = good.clone();
        short.bytes.pop();
        assert!(decode::<Vec<u64>>(&short).is_err());
        let mut trailing = good;
        trailing.floats.push(0);
        assert!(decode::<Vec<u64>>(&trailing).is_err());
    }
}
