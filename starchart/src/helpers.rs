use serde::{Deserialize, Serialize};

/// The key type for [`Values`].
///
/// [`Values`]: crate::Value
pub trait Key: Serialize + for<'de> Deserialize<'de> + Copy {}

/// The [`Value`] type to be used by [`StarMap`].
///
/// [`StarMap`]: crate::StarMap
pub trait Value: Serialize + for<'de> Deserialize<'de> + Clone {
	/// The type of [`Key`] used to index the [`Value`].
	type Key: Key;

	/// Returns the key to index the value of.
	fn key(&self) -> Self::Key;

	/// todo
	fn new(key: Self::Key) -> Self;
}
