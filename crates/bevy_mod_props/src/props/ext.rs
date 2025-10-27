//! Contains extension traits for using props with bevy

use std::sync::LazyLock;

use bevy_ecs::{
    system::{Commands, EntityCommands},
    world::{DeferredWorld, EntityRef, EntityWorldMut, World},
};
use ustr::Ustr;

use super::{Props, Value, ValueError};

pub trait PropsExt {
    /// Returns a read-only set of properties assoceated with this object.
    fn props(&self) -> &Props;
}

static EMPTY_PROPS: LazyLock<Props> = LazyLock::new(Props::default);

impl PropsExt for World {
    fn props(&self) -> &Props {
        match self.get_resource::<Props>() {
            Some(p) => p,
            None => &*EMPTY_PROPS,
        }
    }
}

impl<'w> PropsExt for DeferredWorld<'w> {
    fn props(&self) -> &Props {
        match self.get_resource::<Props>() {
            Some(p) => p,
            None => &*EMPTY_PROPS,
        }
    }
}

impl<'w> PropsExt for EntityRef<'w> {
    fn props(&self) -> &Props {
        match self.get::<Props>() {
            Some(p) => p,
            None => &*EMPTY_PROPS,
        }
    }
}

impl<'w> PropsExt for EntityWorldMut<'w> {
    fn props(&self) -> &Props {
        match self.get::<Props>() {
            Some(p) => p,
            None => &*EMPTY_PROPS,
        }
    }
}

pub trait PropsMutExt {
    /// Provides mutable access to the set of properties assoceated with this object.
    fn props_mut(&mut self) -> &mut Props;
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

pub trait PropAccessExt {
    /// Gets a property value assoceated with this object. If a value exists but
    /// is of the wrong type, an error will be returned.
    fn get_prop<T>(&self, name: impl Into<Ustr>) -> Option<Result<T, ValueError>>
    where
        T: TryFrom<Value, Error = ValueError>;

    /// Gets a property value assoceated with this object, returning whatever
    /// type is avalible.
    fn get_prop_value(&self, name: impl Into<Ustr>) -> Option<Value>;
}

impl<P: PropsExt> PropAccessExt for P {
    fn get_prop<T>(&self, name: impl Into<Ustr>) -> Option<Result<T, ValueError>>
    where
        T: TryFrom<Value, Error = ValueError>,
    {
        self.props().get(name)
    }

    fn get_prop_value(&self, name: impl Into<Ustr>) -> Option<Value> {
        self.props().get_value(name)
    }
}

pub trait PropMutateExt {
    /// Sets a property assoceated with this object.
    fn set_prop(&mut self, name: impl Into<Ustr>, value: impl Into<Value>) -> &mut Self;

    /// Removes a property from this object.
    fn remove_prop(&mut self, name: impl Into<Ustr>) -> &mut Self;

    /// Clears all properties on this object.
    fn clear_props(&mut self) -> &mut Self;
}

impl<P: PropsMutExt> PropMutateExt for P {
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

impl<'w, 's> PropMutateExt for Commands<'w, 's> {
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

impl<'a> PropMutateExt for EntityCommands<'a> {
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
