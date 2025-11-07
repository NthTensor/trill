//! Contains extension traits for using props with bevy

use bevy_ecs::{
    system::{Commands, EntityCommands},
    world::{DeferredWorld, EntityRef, EntityWorldMut, World},
};
use ustr::Ustr;

use super::{DefaultRef, Props, Value};

// -----------------------------------------------------------------------------
// Core immutable properties access

/// Adds [`Props`] access to [`World`], [`DeferredWorld`], [`EntityRef`], and
/// [`EntityWorldMut`].
pub trait PropsExt {
    /// Returns a read-only set of properties assoceated with this object.
    fn props(&self) -> &Props;

    /// Returns an immutable reference to a property value. If the property is
    /// of the wrong type or is not set, a reference to a default value will be
    /// returned instead.
    fn get_prop<T>(&self, name: impl Into<Ustr>) -> &T
    where
        T: DefaultRef + 'static,
        Value: AsRef<T>,
    {
        self.props().get(name)
    }
}

impl PropsExt for World {
    fn props(&self) -> &Props {
        match self.get_resource::<Props>() {
            Some(p) => p,
            None => Props::default_ref(),
        }
    }
}

impl<'w> PropsExt for DeferredWorld<'w> {
    fn props(&self) -> &Props {
        match self.get_resource::<Props>() {
            Some(p) => p,
            None => Props::default_ref(),
        }
    }
}

impl<'w> PropsExt for EntityRef<'w> {
    fn props(&self) -> &Props {
        match self.get::<Props>() {
            Some(p) => p,
            None => Props::default_ref(),
        }
    }
}

impl<'w> PropsExt for EntityWorldMut<'w> {
    fn props(&self) -> &Props {
        match self.get::<Props>() {
            Some(p) => p,
            None => Props::default_ref(),
        }
    }
}

// -----------------------------------------------------------------------------
// Core mutable properties access

/// Adds mutable [`Props`] access to [`World`] and [`EntityWorldMut`].
pub trait PropsMutExt {
    /// Provides mutable access to the set of properties assoceated with this object.
    fn props_mut(&mut self) -> &mut Props;

    /// Returns a mutable reference to a property value. If the propety value is
    /// of the wrong type or not set, a default value of the correct type will
    /// be inserted.
    fn get_prop_mut<T>(&mut self, name: impl Into<Ustr>) -> &mut T
    where
        Value: AsMut<T>,
    {
        self.props_mut().get_mut(name)
    }
}

impl PropsMutExt for World {
    fn props_mut(&mut self) -> &mut Props {
        self.get_resource_or_init::<Props>().into_inner()
    }
}

impl<'w> PropsMutExt for EntityWorldMut<'w> {
    fn props_mut(&mut self) -> &mut Props {
        self.entry::<Props>().or_default().into_mut().into_inner()
    }
}

// -----------------------------------------------------------------------------
// Property commands

/// Adds property mutations to [`Commands`], [`EntityCommands`],
/// [`World`] and [`EntityWorldMut`].
pub trait PropCommandsExt {
    /// Sets a property assoceated with this object.
    fn set_prop(&mut self, name: impl Into<Ustr>, value: impl Into<Value>) -> &mut Self;

    /// Removes a property from this object.
    fn remove_prop(&mut self, name: impl Into<Ustr>) -> &mut Self;

    /// Clears all properties on this object.
    fn clear_props(&mut self) -> &mut Self;
}

impl<P: PropsMutExt> PropCommandsExt for P {
    fn set_prop(&mut self, name: impl Into<Ustr>, value: impl Into<Value>) -> &mut Self {
        self.props_mut().set(name, value);
        self
    }

    fn remove_prop(&mut self, name: impl Into<Ustr>) -> &mut Self {
        self.props_mut().remove(name);
        self
    }

    fn clear_props(&mut self) -> &mut Self {
        self.props_mut().clear();
        self
    }
}

impl<'w, 's> PropCommandsExt for Commands<'w, 's> {
    fn set_prop(&mut self, name: impl Into<Ustr>, value: impl Into<Value>) -> &mut Self {
        let name = name.into();
        let value = value.into();
        self.queue(move |world: &mut World| {
            world.set_prop(name, value);
        });
        self
    }

    fn remove_prop(&mut self, name: impl Into<Ustr>) -> &mut Self {
        let name = name.into();
        self.queue(move |world: &mut World| {
            world.remove_prop(name);
        });
        self
    }

    fn clear_props(&mut self) -> &mut Self {
        self.queue(|world: &mut World| {
            world.clear_props();
        });
        self
    }
}

impl<'a> PropCommandsExt for EntityCommands<'a> {
    fn set_prop(&mut self, name: impl Into<Ustr>, value: impl Into<Value>) -> &mut Self {
        let name = name.into();
        let value = value.into();
        self.queue(move |mut entity: EntityWorldMut| {
            entity.set_prop(name, value);
        });
        self
    }

    fn remove_prop(&mut self, name: impl Into<Ustr>) -> &mut Self {
        let name = name.into();
        self.queue(move |mut entity: EntityWorldMut| {
            entity.remove_prop(name);
        });
        self
    }

    fn clear_props(&mut self) -> &mut Self {
        self.queue(|mut entity: EntityWorldMut| {
            entity.clear_props();
        });
        self
    }
}
