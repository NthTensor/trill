//! Provides a simple key-value property system for bevy, modeled off the source-engine.
//!
//! A property is just a named variable, attached to either an entity or the
//! world. Properties can have booleans, numbers, and strings as values.
//!
//! ```rust
//! // you can set properties on the world using the PropMutateExt extension trait.
//! world.set_prop("is_raining", true);
//! world.set_prop("rain_amount", 2.3);
//! world.set_prop("light_level", "shadow");
//!
//! // you can also set properties on entities using PropMutateExt
//! world.spawn(Transform::default())
//!     .set_prop("distance_to_player", 70.0);
//! ```
//!
//! Entities may also be given a name and class, which can be used for lookups.
//!
//! ```rust
//! // insert a new named entity into the world
//! world.spawn_empty(Health(10.0))
//!     .set_name("legolas")
//!     .set_class("party_member");
//!
//! // later retrive that entity
//! world.entity_named("legolas").get::<Health>();
//! ```
//!
//! No plugins are required to use this crate. Just start inserting components
//! or using the extension traits.

mod props;
pub use props::*;

#[cfg(feature = "bevy")]
mod registry;

#[cfg(feature = "bevy")]
pub use registry::*;
