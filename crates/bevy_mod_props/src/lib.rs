//! Provides a simple key-value property system for bevy, modeled off the source-engine.
//!
//! # Properties
//!
//! This crate provides a "property set" construct called `Props`. Properties
//! are named values. The `Props` is basically just a `BTree<Ustr, Value>` where
//! `Ustr` is an interned string and [`Value`] contains either a `bool`, `f32`,
//! or `Ustr`.
//!
//! ```rust
//! # use bevy_mod_props::*;
//!
//! // props can be created and set like any other rust collection type
//! let mut props = Props::new();
//! props.set("bool_prop", true);
//! props.set("num_prop", 42.0);
//! props.set("str_prop", "string");
//!
//! // for convinence, there is also a `with` method to allow in-place construction
//! let mut props = Props::new()
//!     .with("bool_prop", true)
//!     .with("num_prop", 42.0)
//!     .with("str_prop", "string");
//!
//! // props automatically convert to the desitred type when accessed
//! assert_eq!(props["bool_prop"], true);
//! assert_eq!(props["num_prop"], 42.0);
//! assert_eq!(props["str_prop"], "string");
//!
//! // when the prop dosn't exist, or is the wrong type, the default is returned instead
//! assert_eq!(props["non_existant"], false);
//! let num: f32 = props.get("str_prop");
//! assert_eq!(num, 0.0);
//!
//! // mutable access is also possible
//! let str_prop = props.get_mut("str_prop");
//! *str_prop = Ustr::from("hello world");
//! props["prop_2"] -= 32.0;
//!
//! // mutable access inserts a default if the value dosn't exist or is the wrong type, then returns a reference
//! props["str_prop"] += 10.0;
//! assert_eq!(props["str_prop"], 10.0);
//! ```
//!
//! Props are designed to be easy to read and write, and generally prioritizes
//! ergonomics over explicet error handling.
//!
//! When used with bevy, properties can be either set globally (by accesing
//! `Props` as a resource) or per-entity (by accessing `Props` as a component).
//!
//! ```rust
//! # use bevy_mod_props::*;
//! # use bevy_ecs::prelude::*;
//!
//! fn props_resource_system(props: Res<Props>) {
//!     props.get("thingy");
//! }
//!
//! fn props_world_system(world: &mut World) {
//! }
//! ```
//!
//! # Names & Classes
//!
//! Entities may also be given a name and class, which can be used for lookups.
//!
//! ```rust
//! # use bevy_ecs::prelude::*;
//! # use bevy_mod_props::*;
//! #
//! # fn system(world: &mut World) -> Result {
//! #
//! // insert a new named entity into the world
//! world.spawn_empty()
//!     .set_name("legolas")
//!     .set_class("party_member");
//!
//! // names can also be inserted as components
//! world.spawn((
//!     Identity::new("gimli"),
//!     Class::new("party_member")
//! ));
//!
//! // later retrive that entity to update it's props
//! world.entity_mut_named("legolas")?
//!     .set_prop("equiped", "elven_knives");
//!
//! // or iterate through all members of class
//! for mut party_member in world.entity_mut_class("party_member") {
//!     party_member
//!         .set_prop("has_seen_ringwraith", true);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Names are unique: only one entity may use a given name at a time. Classes
//! are non-unique, but each entity may only have one class.

mod props;
pub use props::*;

#[cfg(feature = "bevy")]
mod registry;

#[cfg(feature = "bevy")]
pub use registry::*;
