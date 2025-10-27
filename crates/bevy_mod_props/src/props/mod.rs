//! Defines the core props datatype.

use std::collections::btree_map::*;
use std::fmt;

use ustr::Ustr;

#[cfg(feature = "bevy")]
use bevy_ecs::component::Component;

#[cfg(feature = "bevy")]
use bevy_ecs::resource::Resource;

#[cfg(feature = "bevy")]
mod ext;

#[cfg(feature = "bevy")]
pub use ext::*;

/// A weakly typed value, for use with properties.
///
/// Values may be either a boolean, number, or string. You can use `Into/From` to
/// convert from normal rust datatypes into values, and `TryInto/TryFrom` to convert back.
#[derive(Debug, Copy, Clone)]
pub enum Value {
    Bool(bool),
    Num(f32),
    Str(Ustr),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool(bool) => write!(f, "{}", bool),
            Value::Num(num) => write!(f, "{}", num),
            Value::Str(ustr) => write!(f, "{}", ustr),
        }
    }
}

impl Into<Value> for bool {
    fn into(self) -> Value {
        Value::Bool(self)
    }
}

impl Into<Value> for f32 {
    fn into(self) -> Value {
        Value::Num(self)
    }
}

impl Into<Value> for &str {
    fn into(self) -> Value {
        Value::Str(self.into())
    }
}

impl Into<Value> for String {
    fn into(self) -> Value {
        Value::Str(self.into())
    }
}

impl Into<Value> for Ustr {
    fn into(self) -> Value {
        Value::Str(self)
    }
}

#[derive(Debug)]
pub enum ValueError {
    IsBool(bool),
    IsNum(f32),
    IsStr(Ustr),
}

impl TryInto<bool> for Value {
    type Error = ValueError;

    fn try_into(self) -> Result<bool, Self::Error> {
        match self {
            Value::Bool(bool) => Ok(bool),
            Value::Num(num) => Err(ValueError::IsNum(num)),
            Value::Str(ustr) => Err(ValueError::IsStr(ustr)),
        }
    }
}

impl TryInto<f32> for Value {
    type Error = ValueError;

    fn try_into(self) -> Result<f32, Self::Error> {
        match self {
            Value::Bool(bool) => Err(ValueError::IsBool(bool)),
            Value::Num(num) => Ok(num),
            Value::Str(ustr) => Err(ValueError::IsStr(ustr)),
        }
    }
}

impl TryInto<Ustr> for Value {
    type Error = ValueError;

    fn try_into(self) -> Result<Ustr, Self::Error> {
        match self {
            Value::Bool(bool) => Err(ValueError::IsBool(bool)),
            Value::Num(num) => Err(ValueError::IsNum(num)),
            Value::Str(ustr) => Ok(ustr),
        }
    }
}

/// A simple key-value property store, accessable either as a component or a
/// resource.
///
/// Properties have string keys and either boolean, numeric, or string
/// values. It is often more convivient to work through the extension traits
/// [`PropsExt`], [`PropsMutExt`], [`PropAccessExt`], or [`PropMutateExt`].
#[derive(Default)]
#[cfg_attr(feature = "bevy", derive(Component, Resource))]
pub struct Props {
    properties: BTreeMap<Ustr, Value>,
}

impl Props {
    /// Creats a new set of properties. This is done automatically for you when using
    /// the extension traits.
    pub fn new() -> Props {
        Props::default()
    }

    ////Gets a property value. If a value exists but is of the wrong type, an
    /// error will be returned.
    pub fn get<T>(&self, name: impl Into<Ustr>) -> Option<Result<T, ValueError>>
    where
        T: TryFrom<Value, Error = ValueError>,
    {
        self.get_value(name).map(T::try_from)
    }

    /// Gets a property value, returning whatever type is avalible.
    pub fn get_value(&self, name: impl Into<Ustr>) -> Option<Value> {
        self.properties.get(&name.into()).copied()
    }

    /// Sets a property value.
    pub fn set(&mut self, name: impl Into<Ustr>, value: impl Into<Value>) {
        self.properties.insert(name.into(), value.into());
    }

    /// Sets a property value, and can be chained.
    pub fn with(mut self, name: impl Into<Ustr>, value: impl Into<Value>) -> Self {
        self.set(name, value);
        self
    }

    /// Removes a property.
    pub fn remove(&mut self, name: impl Into<Ustr>) {
        self.properties.remove(&name.into());
    }

    /// Clears all properties.
    pub fn clear(&mut self) {
        self.properties.clear();
    }

    pub fn iter(&self) -> Iter<Ustr, Value> {
        self.properties.iter()
    }

    /// Creates a borrowing iterator over property names.
    pub fn keys(&self) -> Keys<Ustr, Value> {
        self.properties.keys()
    }

    /// Creates a consuming iterator over property names.
    pub fn into_keys(self) -> IntoKeys<Ustr, Value> {
        self.properties.into_keys()
    }

    /// Creates a borrowing iterator over property values.
    pub fn values(&self) -> Values<Ustr, Value> {
        self.properties.values()
    }

    /// Creates a consuming iterator over property values.
    pub fn into_values(self) -> IntoValues<Ustr, Value> {
        self.properties.into_values()
    }

    /// Creates a mutable borrowing iterator over property values.
    pub fn values_mut(&mut self) -> ValuesMut<Ustr, Value> {
        self.properties.values_mut()
    }
}

impl IntoIterator for Props {
    type Item = (Ustr, Value);
    type IntoIter = IntoIter<Ustr, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.properties.into_iter()
    }
}
