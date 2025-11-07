//! Defines extension traits for using the registry with bevy

use bevy_ecs::{
    entity::{Entity, EntityHashSet},
    system::EntityCommands,
    world::{DeferredWorld, EntityMut, EntityRef, EntityWorldMut, World},
};
use ustr::Ustr;

use super::{Class, EMPTY_SET, Identity, Registry};

// -----------------------------------------------------------------------------
// Registry Access

pub trait RegistryExt {
    fn get_name(&self) -> Option<Ustr>;

    fn get_class(&self) -> Option<Ustr>;
}

impl<'w> RegistryExt for EntityRef<'w> {
    fn get_name(&self) -> Option<Ustr> {
        self.get::<Identity>().map(|i| i.0)
    }

    fn get_class(&self) -> Option<Ustr> {
        self.get::<Class>().map(|i| i.0)
    }
}

// -----------------------------------------------------------------------------
// Registry Mutation

pub trait RegistryCommandsExt {
    fn set_name(&mut self, name: impl Into<Ustr>) -> &mut Self;

    fn set_class(&mut self, class: impl Into<Ustr>) -> &mut Self;
}

impl<'w> RegistryCommandsExt for EntityWorldMut<'w> {
    fn set_name(&mut self, name: impl Into<Ustr>) -> &mut Self {
        self.insert(Identity::new(name))
    }

    fn set_class(&mut self, class: impl Into<Ustr>) -> &mut Self {
        self.insert(Class::new(class))
    }
}

impl<'w> RegistryCommandsExt for EntityCommands<'w> {
    fn set_name(&mut self, name: impl Into<Ustr>) -> &mut Self {
        self.insert(Identity::new(name))
    }

    fn set_class(&mut self, class: impl Into<Ustr>) -> &mut Self {
        self.insert(Class::new(class))
    }
}

// -----------------------------------------------------------------------------
// Registryx lookups

pub trait RegistryLookupExt {
    fn lookup_name(&self, name: impl Into<Ustr>) -> Option<Entity>;

    fn lookup_class(&self, class: impl Into<Ustr>) -> &EntityHashSet;

    fn entity_named(&self, name: impl Into<Ustr>) -> Option<EntityRef>;
}

impl RegistryLookupExt for World {
    fn lookup_name(&self, name: impl Into<Ustr>) -> Option<Entity> {
        if let Some(registry) = self.get_resource::<Registry>() {
            registry.lookup_name(name)
        } else {
            None
        }
    }

    fn lookup_class(&self, class: impl Into<Ustr>) -> &EntityHashSet {
        if let Some(registry) = self.get_resource::<Registry>() {
            registry.lookup_class(class)
        } else {
            &EMPTY_SET
        }
    }

    fn entity_named(&self, name: impl Into<Ustr>) -> Option<EntityRef> {
        self.lookup_name(name).and_then(|e| self.get_entity(e).ok())
    }
}

impl<'w> RegistryLookupExt for DeferredWorld<'w> {
    fn lookup_name(&self, name: impl Into<Ustr>) -> Option<Entity> {
        if let Some(registry) = self.get_resource::<Registry>() {
            registry.named_entities.get(&name.into()).copied()
        } else {
            None
        }
    }

    fn lookup_class(&self, class: impl Into<Ustr>) -> &EntityHashSet {
        if let Some(registry) = self.get_resource::<Registry>() {
            registry.lookup_class(class)
        } else {
            &EMPTY_SET
        }
    }

    fn entity_named(&self, name: impl Into<Ustr>) -> Option<EntityRef> {
        self.lookup_name(name).and_then(|e| self.get_entity(e).ok())
    }
}

// -----------------------------------------------------------------------------
// Mutable registry lookups lookup

pub trait RegistryLookupMutExt {
    fn entity_mut_named(&mut self, name: impl Into<Ustr>) -> Option<EntityWorldMut>;
}

impl RegistryLookupMutExt for World {
    fn entity_mut_named(&mut self, name: impl Into<Ustr>) -> Option<EntityWorldMut> {
        self.lookup_name(name)
            .and_then(|e| self.get_entity_mut(e).ok())
    }
}

// -----------------------------------------------------------------------------
// Deferred mutable registry lookups

pub trait RegistryLookupDeferredExt {
    fn entity_mut_named(&mut self, name: impl Into<Ustr>) -> Option<EntityMut>;
}

impl<'w> RegistryLookupDeferredExt for DeferredWorld<'w> {
    fn entity_mut_named(&mut self, name: impl Into<Ustr>) -> Option<EntityMut> {
        self.lookup_name(name)
            .and_then(|e| self.get_entity_mut(e).ok())
    }
}
