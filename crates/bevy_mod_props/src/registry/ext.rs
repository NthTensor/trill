//! Defines extension traits for using the registry with bevy

use bevy_ecs::{
    entity::{Entity, EntityDoesNotExistError, EntityHashSet},
    system::EntityCommands,
    world::{
        DeferredWorld, EntityMut, EntityRef, EntityWorldMut, World, WorldEntityFetch,
        error::EntityMutableFetchError, unsafe_world_cell::UnsafeWorldCell,
    },
};
use thiserror::Error;
use ustr::Ustr;

use super::{Class, EMPTY_SET, EntityNotFoundError, Identity, Registry};

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

#[derive(Debug, Error)]
#[error("{0}")]
pub enum EntityNamedError {
    EntityNotFound(#[from] EntityNotFoundError),
    EntityDoesNotExist(#[from] EntityDoesNotExistError),
}

pub trait RegistryLookupExt {
    fn lookup_name(&self, name: impl Into<Ustr>) -> Result<Entity, EntityNotFoundError>;

    fn lookup_class(&self, class: impl Into<Ustr>) -> &EntityHashSet;

    fn entity_named(&self, name: impl Into<Ustr>) -> Result<EntityRef, EntityNamedError>;

    fn entity_class(&self, class: impl Into<Ustr>) -> EntityClassIter;
}

pub struct EntityClassIter<'w> {
    entities: bevy_ecs::entity::hash_set::IntoIter,
    world: &'w World,
}

impl<'w> Iterator for EntityClassIter<'w> {
    type Item = EntityRef<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        let entity = self.entities.next()?;
        Some(self.world.entity(entity))
    }
}

impl RegistryLookupExt for World {
    fn lookup_name(&self, name: impl Into<Ustr>) -> Result<Entity, EntityNotFoundError> {
        if let Some(registry) = self.get_resource::<Registry>() {
            registry.lookup_name(name)
        } else {
            Err(EntityNotFoundError { name: name.into() })
        }
    }

    fn lookup_class(&self, class: impl Into<Ustr>) -> &EntityHashSet {
        if let Some(registry) = self.get_resource::<Registry>() {
            registry.lookup_class(class)
        } else {
            &EMPTY_SET
        }
    }

    fn entity_named(&self, name: impl Into<Ustr>) -> Result<EntityRef, EntityNamedError> {
        let entity = self.lookup_name(name)?;
        let entity_ref = self.get_entity(entity)?;
        Ok(entity_ref)
    }

    fn entity_class(&self, class: impl Into<Ustr>) -> EntityClassIter {
        EntityClassIter {
            entities: self.lookup_class(class).clone().into_iter(),
            world: self,
        }
    }
}

impl<'w> RegistryLookupExt for DeferredWorld<'w> {
    fn lookup_name(&self, name: impl Into<Ustr>) -> Result<Entity, EntityNotFoundError> {
        if let Some(registry) = self.get_resource::<Registry>() {
            registry.lookup_name(name)
        } else {
            Err(EntityNotFoundError { name: name.into() })
        }
    }

    fn lookup_class(&self, class: impl Into<Ustr>) -> &EntityHashSet {
        if let Some(registry) = self.get_resource::<Registry>() {
            registry.lookup_class(class)
        } else {
            &EMPTY_SET
        }
    }

    fn entity_named(&self, name: impl Into<Ustr>) -> Result<EntityRef, EntityNamedError> {
        let entity = self.lookup_name(name)?;
        let entity_ref = self.get_entity(entity)?;
        Ok(entity_ref)
    }

    fn entity_class(&self, class: impl Into<Ustr>) -> EntityClassIter {
        EntityClassIter {
            entities: self.lookup_class(class).clone().into_iter(),
            world: self,
        }
    }
}

// -----------------------------------------------------------------------------
// Mutable registry lookups lookup

#[derive(Debug, Error)]
#[error("{0}")]
pub enum EntityNamedMutError {
    EntityNotFound(#[from] EntityNotFoundError),
    EntityMutableFetchError(#[from] EntityMutableFetchError),
}

pub trait RegistryLookupMutExt {
    fn entity_mut_named(
        &mut self,
        name: impl Into<Ustr>,
    ) -> Result<EntityWorldMut, EntityNamedMutError>;

    fn entity_mut_class(&mut self, class: impl Into<Ustr>) -> EntityClassMutIter;
}

pub struct EntityClassMutIter<'w> {
    entities: bevy_ecs::entity::hash_set::IntoIter,
    world_cell: UnsafeWorldCell<'w>,
}

impl<'w> Iterator for EntityClassMutIter<'w> {
    type Item = EntityWorldMut<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        let entity = self.entities.next()?;
        // SAFETY: TODO
        let entity_mut = unsafe { entity.fetch_mut(self.world_cell).unwrap() };
        Some(entity_mut)
    }
}

impl RegistryLookupMutExt for World {
    fn entity_mut_named(
        &mut self,
        name: impl Into<Ustr>,
    ) -> Result<EntityWorldMut, EntityNamedMutError> {
        let entity = self.lookup_name(name)?;
        let entity_mut = self.get_entity_mut(entity)?;
        Ok(entity_mut)
    }

    fn entity_mut_class(&mut self, class: impl Into<Ustr>) -> EntityClassMutIter {
        EntityClassMutIter {
            entities: self.lookup_class(class).clone().into_iter(),
            world_cell: self.as_unsafe_world_cell(),
        }
    }
}

// -----------------------------------------------------------------------------
// Deferred mutable registry lookups

pub trait RegistryLookupDeferredExt {
    fn entity_mut_named(&mut self, name: impl Into<Ustr>)
    -> Result<EntityMut, EntityNamedMutError>;

    fn entity_mut_class(&mut self, class: impl Into<Ustr>) -> EntityClassDeferredIter;
}

impl<'w> RegistryLookupDeferredExt for DeferredWorld<'w> {
    fn entity_mut_named(
        &mut self,
        name: impl Into<Ustr>,
    ) -> Result<EntityMut, EntityNamedMutError> {
        let entity = self.lookup_name(name)?;
        let entity_mut = self.get_entity_mut(entity)?;
        Ok(entity_mut)
    }

    fn entity_mut_class(&mut self, class: impl Into<Ustr>) -> EntityClassDeferredIter {
        EntityClassDeferredIter {
            entities: self.lookup_class(class).clone().into_iter(),
            world_cell: self.as_unsafe_world_cell_readonly(),
        }
    }
}

pub struct EntityClassDeferredIter<'w> {
    entities: bevy_ecs::entity::hash_set::IntoIter,
    world_cell: UnsafeWorldCell<'w>,
}

impl<'w> Iterator for EntityClassDeferredIter<'w> {
    type Item = EntityMut<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        let entity = self.entities.next()?;
        // SAFETY: TODO
        let entity_mut = unsafe { entity.fetch_deferred_mut(self.world_cell).unwrap() };
        Some(entity_mut)
    }
}
