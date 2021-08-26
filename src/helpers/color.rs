use serde::{
	de::{Error as DeError, Visitor},
	Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt::{Formatter, Result as FmtResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Color(u8, u8, u8);

impl Color {
	pub const fn new(r: u8, g: u8, b: u8) -> Self {
		Self(r, g, b)
	}

	pub const fn r(self) -> u8 {
		self.0
	}

	pub const fn g(self) -> u8 {
		self.1
	}

	pub const fn b(self) -> u8 {
		self.2
	}

	pub const fn to_decimal(self) -> u32 {
		u32::from_be_bytes([0, self.r(), self.g(), self.b()])
	}

	pub const fn from_decimal(decimal: u32) -> Self {
		let [_, r, g, b] = decimal.to_be_bytes();
		Self(r, g, b)
	}
}

impl Default for Color {
	fn default() -> Self {
		Self(255, 255, 255)
	}
}

impl Serialize for Color {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_u32(self.to_decimal())
	}
}

struct ColorVisitor;

impl<'de> Visitor<'de> for ColorVisitor {
	type Value = Color;

	fn expecting(&self, formatter: &mut Formatter) -> FmtResult {
		formatter.write_str("a valid u32")
	}

	fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
	where
		E: DeError,
	{
		Ok(Color::from_decimal(
			v.parse::<u32>().map_err(DeError::custom)?,
		))
	}

	fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
	where
		E: DeError,
	{
		Ok(Color::from_decimal(v.into()))
	}

	fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
	where
		E: DeError,
	{
		Ok(Color::from_decimal(v.into()))
	}

	fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
	where
		E: DeError,
	{
		Ok(Color::from_decimal(v))
	}
}

impl<'de> Deserialize<'de> for Color {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_u32(ColorVisitor)
	}
}

#[cfg(test)]
mod tests {
	use super::Color;
	use serde::{Deserialize, Serialize};
	use static_assertions::assert_impl_all;
	use std::{fmt::Debug, hash::Hash};

	assert_impl_all!(
		Color: Clone,
		Copy,
		Debug,
		Default,
		Deserialize<'static>,
		Eq,
		Hash,
		Ord,
		PartialEq,
		PartialOrd,
		Send,
		Serialize,
		Sync
	);

	#[test]
	fn from_decimal() {
		let decimal = 16777215;
		let expected = Color::new(255, 255, 255);

		assert_eq!(Color::from_decimal(decimal), expected);
	}

	#[test]
	fn to_decimal() {
		let color = Color::new(255, 255, 255);
		let expected = 16777215;

		assert_eq!(color.to_decimal(), expected);
	}
}
