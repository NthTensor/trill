//! Defines the core props datatype.

use std::collections::btree_map::*;
use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Sub, SubAssign};
use std::sync::LazyLock;

pub use ustr::Ustr;

#[cfg(feature = "bevy")]
use bevy_ecs::component::Component;

#[cfg(feature = "bevy")]
use bevy_ecs::resource::Resource;

#[cfg(feature = "bevy")]
mod ext;

#[cfg(feature = "bevy")]
pub use ext::*;

// -----------------------------------------------------------------------------
// The Value Type

/// A weakly typed value, for use with properties.
///
/// Values may be either a boolean, number, or string. You can use `Into/From` to
/// convert from normal rust datatypes into values, and `TryInto/TryFrom` to
/// convert back. Using `TryFrom` will return an error if the types do not
/// match.
///
/// # Value Access
///
/// For better ergonomics, the [`AsRef`] and [`AsMut`] allow accessing a value
/// _as if it were of a given type_. If the types do not actually match, the
/// default value of the requested type will be used instead.
///
/// ```rust
/// # use bevy_mod_props::Value;
/// let mut my_value = Value::from("hello");
///
/// // I can use `as_ref` to treat the value as a boolean
/// let my_value_bool: &bool = my_value.as_ref();
/// assert_eq!(*my_value_bool, false);
///
/// // I can use `as_mut` to turn the value as mutable float
/// let my_float_value: &mut f32 = my_value.as_mut();
/// *my_float_value += 10.0;
///
/// // Now the value will be 10.0
/// assert_eq!(my_value, Value::Num(10.0))
/// ```
///
/// # Equality
///
/// Two values are equal if they contain equal values of the same type. Values
/// with different types are never equal. `Value::num(NaN)` is equal to nothing.
///
/// # Math
///
/// `Value` supports the basic algebraic operations: [`Add`], [`Sub`], [`Mul`],
/// and [`Div`]. Values that do not contain numbers always act like zero, except
/// in the case of devision. In the expression `ValueA / ValueB`, if `ValueB` is
/// not a number, the result is `ValueA` rather than `NaN`. If neither are
/// numbers, the result is zero.
///
/// Doing any kind of math with `Value` always returns a `Value::Num` variant.
#[derive(Debug, Copy, Clone)]
pub enum Value {
    Bool(bool),
    Num(f32),
    Str(Ustr),
}

// -----------------------------------------------------------------------------
// Defaults

impl Default for Value {
    fn default() -> Self {
        Value::Bool(false)
    }
}

// -----------------------------------------------------------------------------
// Printing

impl fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool(bool) => write!(f, "{bool}"),
            Value::Num(num) => write!(f, "{num}"),
            Value::Str(ustr) => write!(f, "{ustr}"),
        }
    }
}

// -----------------------------------------------------------------------------
// Type Conversion

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value::Num(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::Str(value.into())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::Str(value.into())
    }
}

impl From<Ustr> for Value {
    fn from(value: Ustr) -> Self {
        Value::Str(value)
    }
}

impl From<Value> for bool {
    fn from(value: Value) -> Self {
        match value {
            Value::Bool(bool) => bool,
            _ => false,
        }
    }
}

impl From<Value> for f32 {
    fn from(value: Value) -> Self {
        match value {
            Value::Num(num) => num,
            _ => 0.0,
        }
    }
}

impl From<Value> for f64 {
    fn from(value: Value) -> Self {
        match value {
            Value::Num(num) => num as f64,
            _ => 0.0,
        }
    }
}

impl From<Value> for &str {
    fn from(value: Value) -> Self {
        match value {
            Value::Str(str) => str.as_str(),
            _ => "",
        }
    }
}

impl From<Value> for Ustr {
    fn from(value: Value) -> Self {
        match value {
            Value::Str(str) => str,
            _ => Ustr::from(""),
        }
    }
}

// -----------------------------------------------------------------------------
// Referencing and casting

impl AsRef<bool> for Value {
    fn as_ref(&self) -> &bool {
        match self {
            Value::Bool(bool) => bool,
            _ => &false,
        }
    }
}

impl AsRef<f32> for Value {
    fn as_ref(&self) -> &f32 {
        match self {
            Value::Num(num) => num,
            _ => &0.0,
        }
    }
}

static EMPTY_USTR: LazyLock<Ustr> = LazyLock::new(|| Ustr::from(""));

impl AsRef<Ustr> for Value {
    fn as_ref(&self) -> &Ustr {
        match self {
            Value::Str(str) => str,
            _ => &EMPTY_USTR,
        }
    }
}

impl AsMut<bool> for Value {
    fn as_mut(&mut self) -> &mut bool {
        match self {
            Value::Bool(bool) => bool,
            _ => {
                *self = Value::Bool(false);
                let Value::Bool(bool) = self else {
                    unreachable!();
                };
                bool
            }
        }
    }
}

impl AsRef<Value> for Value {
    fn as_ref(&self) -> &Value {
        self
    }
}

impl AsMut<f32> for Value {
    fn as_mut(&mut self) -> &mut f32 {
        match self {
            Value::Num(num) => num,
            _ => {
                *self = Value::Num(0.0);
                let Value::Num(num) = self else {
                    unreachable!();
                };
                num
            }
        }
    }
}

impl AsMut<Ustr> for Value {
    fn as_mut(&mut self) -> &mut Ustr {
        match self {
            Value::Str(str) => str,
            _ => {
                *self = Value::Str(Ustr::from(""));
                let Value::Str(str) = self else {
                    unreachable!();
                };
                str
            }
        }
    }
}

impl AsMut<Value> for Value {
    fn as_mut(&mut self) -> &mut Value {
        self
    }
}

// -----------------------------------------------------------------------------
// Equality

impl PartialEq<bool> for Value {
    fn eq(&self, other: &bool) -> bool {
        match self {
            Value::Bool(this) => this == other,
            _ => false,
        }
    }
}

impl PartialEq<Value> for bool {
    fn eq(&self, other: &Value) -> bool {
        match other {
            Value::Bool(that) => self == that,
            _ => false,
        }
    }
}

impl PartialEq<f32> for Value {
    fn eq(&self, other: &f32) -> bool {
        match self {
            Value::Num(this) => this == other,
            _ => false,
        }
    }
}

impl PartialEq<Value> for f32 {
    fn eq(&self, other: &Value) -> bool {
        match other {
            Value::Num(that) => self == that,
            _ => false,
        }
    }
}

impl PartialEq<&str> for Value {
    fn eq(&self, other: &&str) -> bool {
        match self {
            Value::Str(this) => *this == Ustr::from(other),
            _ => false,
        }
    }
}

impl PartialEq<Value> for &str {
    fn eq(&self, other: &Value) -> bool {
        match other {
            Value::Str(that) => *self == Ustr::from(that),
            _ => false,
        }
    }
}

impl PartialEq<String> for Value {
    fn eq(&self, other: &String) -> bool {
        match self {
            Value::Str(this) => *this == Ustr::from(other),
            _ => false,
        }
    }
}

impl PartialEq<Value> for String {
    fn eq(&self, other: &Value) -> bool {
        match other {
            Value::Str(that) => *self == Ustr::from(that),
            _ => false,
        }
    }
}

impl PartialEq<Ustr> for Value {
    fn eq(&self, other: &Ustr) -> bool {
        match self {
            Value::Str(this) => this == other,
            _ => false,
        }
    }
}

impl PartialEq<Value> for Ustr {
    fn eq(&self, other: &Value) -> bool {
        match other {
            Value::Str(that) => self == that,
            _ => false,
        }
    }
}

impl PartialEq<Value> for Value {
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Bool(this), Value::Bool(that)) => this == that,
            (Value::Num(this), Value::Num(that)) => this == that,
            (Value::Str(this), Value::Str(that)) => this == that,
            _ => false,
        }
    }
}

impl Eq for Value {}

// -----------------------------------------------------------------------------
// Comparison

impl PartialOrd<bool> for Value {
    fn partial_cmp(&self, that: &bool) -> Option<std::cmp::Ordering> {
        match self {
            Value::Bool(this) => this.partial_cmp(that),
            _ => None,
        }
    }
}

impl PartialOrd<Value> for bool {
    fn partial_cmp(&self, other: &Value) -> Option<std::cmp::Ordering> {
        match other {
            Value::Bool(that) => self.partial_cmp(that),
            _ => None,
        }
    }
}

impl PartialOrd<f32> for Value {
    fn partial_cmp(&self, that: &f32) -> Option<std::cmp::Ordering> {
        match self {
            Value::Num(this) => this.partial_cmp(that),
            _ => None,
        }
    }
}

impl PartialOrd<Value> for f32 {
    fn partial_cmp(&self, other: &Value) -> Option<std::cmp::Ordering> {
        match other {
            Value::Num(that) => self.partial_cmp(that),
            _ => None,
        }
    }
}

impl PartialOrd<Ustr> for Value {
    fn partial_cmp(&self, that: &Ustr) -> Option<std::cmp::Ordering> {
        match self {
            Value::Str(this) => this.partial_cmp(that),
            _ => None,
        }
    }
}

impl PartialOrd<Value> for Ustr {
    fn partial_cmp(&self, other: &Value) -> Option<std::cmp::Ordering> {
        match other {
            Value::Str(that) => self.partial_cmp(that),
            _ => None,
        }
    }
}

impl PartialOrd<Value> for Value {
    fn partial_cmp(&self, other: &Value) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Bool(this), Value::Bool(that)) => this.partial_cmp(that),
            (Value::Num(this), Value::Num(that)) => this.partial_cmp(that),
            (Value::Str(this), Value::Str(that)) => this.partial_cmp(that),
            _ => None,
        }
    }
}

// -----------------------------------------------------------------------------
// Addition

// Addition is defined for all values. Values that do not contain numbers behave
// as if they contained zero.

impl Add<f32> for Value {
    type Output = Value;

    fn add(self, rhs: f32) -> Self::Output {
        match self {
            Value::Num(lhs) => Value::Num(lhs + rhs),
            _ => Value::Num(rhs),
        }
    }
}

impl Add<Value> for f32 {
    type Output = Value;

    fn add(self, rhs: Value) -> Self::Output {
        match rhs {
            Value::Num(rhs) => Value::Num(self + rhs),
            _ => Value::Num(self),
        }
    }
}

impl AddAssign<f32> for Value {
    fn add_assign(&mut self, rhs: f32) {
        *self = *self + rhs
    }
}

impl Add<Value> for Value {
    type Output = Value;

    fn add(self, rhs: Value) -> Self::Output {
        match (self, rhs) {
            (Value::Num(lhs), Value::Num(rhs)) => Value::Num(lhs + rhs),
            (Value::Num(num), _) | (_, Value::Num(num)) => Value::Num(num),
            _ => Value::Num(0.0),
        }
    }
}

impl AddAssign<Value> for Value {
    fn add_assign(&mut self, rhs: Value) {
        *self = *self + rhs
    }
}

// -----------------------------------------------------------------------------
// Subtraction

// Subtraction is defined for all values. Values that do not contain numbers behave
// as if they contained zero.

impl Sub<f32> for Value {
    type Output = Value;

    fn sub(self, rhs: f32) -> Self::Output {
        match self {
            Value::Num(lhs) => Value::Num(lhs - rhs),
            _ => Value::Num(-rhs),
        }
    }
}

impl Sub<Value> for f32 {
    type Output = Value;

    fn sub(self, rhs: Value) -> Self::Output {
        match rhs {
            Value::Num(rhs) => Value::Num(self - rhs),
            _ => Value::Num(-self),
        }
    }
}

impl SubAssign<f32> for Value {
    fn sub_assign(&mut self, rhs: f32) {
        *self = *self - rhs
    }
}

impl Sub<Value> for Value {
    type Output = Value;

    fn sub(self, rhs: Value) -> Self::Output {
        match (self, rhs) {
            (Value::Num(lhs), Value::Num(rhs)) => Value::Num(lhs - rhs),
            (Value::Num(num), _) => Value::Num(num),
            (_, Value::Num(num)) => Value::Num(-num),
            _ => Value::Num(0.0),
        }
    }
}

impl SubAssign<Value> for Value {
    fn sub_assign(&mut self, rhs: Value) {
        *self = *self - rhs
    }
}

// -----------------------------------------------------------------------------
// Multiplication

// Multiplication is defined for all values. Values that do not contain numbers behave
// as if they contained zero.

impl Mul<f32> for Value {
    type Output = Value;

    fn mul(self, rhs: f32) -> Self::Output {
        match self {
            Value::Num(lhs) => Value::Num(lhs * rhs),
            _ => Value::Num(0.0),
        }
    }
}

impl Mul<Value> for f32 {
    type Output = Value;

    fn mul(self, rhs: Value) -> Self::Output {
        match rhs {
            Value::Num(rhs) => Value::Num(self * rhs),
            _ => Value::Num(0.0),
        }
    }
}

impl MulAssign<f32> for Value {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs
    }
}

impl Mul<Value> for Value {
    type Output = Value;

    fn mul(self, rhs: Value) -> Self::Output {
        match (self, rhs) {
            (Value::Num(lhs), Value::Num(rhs)) => Value::Num(lhs * rhs),
            _ => Value::Num(0.0),
        }
    }
}

impl MulAssign<Value> for Value {
    fn mul_assign(&mut self, rhs: Value) {
        *self = *self * rhs
    }
}

// -----------------------------------------------------------------------------
// Division

// Multiplication is defined for all values. Values that do not contain numbers
// behave as if they contained zero, with one exception: division by a
// non-numeric value is the same as dividing by 1 (it has no effect) rather than
// dividing by 0 (returning NaN).

impl Div<f32> for Value {
    type Output = Value;

    fn div(self, rhs: f32) -> Self::Output {
        match self {
            Value::Num(lhs) => Value::Num(lhs / rhs),
            _ => Value::Num(0.0),
        }
    }
}

impl Div<Value> for f32 {
    type Output = Value;

    fn div(self, rhs: Value) -> Self::Output {
        match rhs {
            Value::Num(rhs) => Value::Num(self / rhs),
            _ => Value::Num(self),
        }
    }
}

impl DivAssign<f32> for Value {
    fn div_assign(&mut self, rhs: f32) {
        *self = *self / rhs
    }
}

impl Div<Value> for Value {
    type Output = Value;

    fn div(self, rhs: Value) -> Self::Output {
        match (self, rhs) {
            (Value::Num(lhs), Value::Num(rhs)) => Value::Num(lhs / rhs),
            (Value::Num(lhs), _) => Value::Num(lhs),
            (_, Value::Num(_)) => Value::Num(0.0),
            _ => Value::Num(0.0),
        }
    }
}

impl DivAssign<Value> for Value {
    fn div_assign(&mut self, rhs: Value) {
        *self = *self / rhs
    }
}

// -----------------------------------------------------------------------------
// Property Maps

/// A simple key-value property store, accessable either as a component or a
/// resource.
///
/// Properties have string keys and either boolean, numeric, or string
/// values. It is often more convivient to work through the extension traits
/// [`PropsExt`], [`PropsMutExt`], and [`PropCommandsExt`].
///
/// When accessing a property, if a value has not been set or has the wrong
/// type, the property should be treated as if it has the default value of the
/// correct type. For example, toggling a
#[derive(Default, Clone, Debug)]
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

    /// Gets the given key’s corresponding entry in the map for in-place manipulation.
    pub fn entry(&mut self, name: impl Into<Ustr>) -> Entry<Ustr, Value> {
        self.properties.entry(name.into())
    }

    /// Returns an immutable reference to a property value. If the property is
    /// of the wrong type or is not set, a reference to a default value will be
    /// returned instead.
    pub fn get<T>(&self, name: impl Into<Ustr>) -> T
    where
        T: From<Value> + Default + 'static,
    {
        if let Some(&value) = self.properties.get(&name.into()) {
            value.into()
        } else {
            T::default()
        }
    }

    /// Returns a mutable reference to a property value. If the propety value is
    /// of the wrong type or not set, a default value of the correct type will
    /// be inserted.
    pub fn get_mut<T>(&mut self, name: impl Into<Ustr>) -> &mut T
    where
        Value: AsMut<T>,
    {
        self.properties.entry(name.into()).or_default().as_mut()
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

    ////Removes a property. Subsiquently accessing this property with `get` or
    /// `get_mut` will return a default value.
    pub fn remove(&mut self, name: impl Into<Ustr>) {
        self.properties.remove(&name.into());
    }

    /// Clears all properties.
    pub fn clear(&mut self) {
        self.properties.clear();
    }

    /// Creates a borrowing iterator over all property names and values.
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

static DEFAULT_VALUE: LazyLock<Value> = LazyLock::new(Value::default);

impl<S: Into<Ustr>> Index<S> for Props {
    type Output = Value;

    fn index(&self, index: S) -> &Self::Output {
        self.properties.get(&index.into()).unwrap_or(&DEFAULT_VALUE)
    }
}

impl<S: Into<Ustr>> IndexMut<S> for Props {
    fn index_mut(&mut self, index: S) -> &mut Self::Output {
        self.get_mut(index)
    }
}

impl IntoIterator for Props {
    type Item = (Ustr, Value);
    type IntoIter = IntoIter<Ustr, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.properties.into_iter()
    }
}
