//! Defines extension traits for using the registry with bevy

use bevy_ecs::{
    entity::{Entity, EntityHashSet},
    world::{DeferredWorld, EntityMut, EntityRef, EntityWorldMut, World},
};
use ustr::Ustr;

use super::{Class, EMPTY_SET, Identity, Registry};

pub trait RegistryAccessExt {
    fn get_name(&self) -> Option<Ustr>;

    fn get_class(&self) -> Option<Ustr>;
}

impl<'w> RegistryAccessExt for EntityRef<'w> {
    fn get_name(&self) -> Option<Ustr> {
        self.get::<Identity>().map(|i| i.0)
    }

    fn get_class(&self) -> Option<Ustr> {
        self.get::<Class>().map(|i| i.0)
    }
}

pub trait RegistryMutateExt {
    fn set_name(&mut self, name: impl Into<Ustr>) -> &mut Self;

    fn set_class(&mut self, class: impl Into<Ustr>) -> &mut Self;
}

impl<'w> RegistryMutateExt for EntityWorldMut<'w> {
    fn set_name(&mut self, name: impl Into<Ustr>) -> &mut Self {
        self.insert(Identity::new(name))
    }

    fn set_class(&mut self, class: impl Into<Ustr>) -> &mut Self {
        self.insert(Class::new(class))
    }
}

pub trait RegistryWorldExt {
    fn lookup_name(&self, name: impl Into<Ustr>) -> Option<Entity>;

    fn lookup_class(&self, class: impl Into<Ustr>) -> &EntityHashSet;

    fn entity_named(&self, name: impl Into<Ustr>) -> Option<EntityRef>;
}

impl RegistryWorldExt for World {
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
            &*EMPTY_SET
        }
    }

    fn entity_named(&self, name: impl Into<Ustr>) -> Option<EntityRef> {
        self.lookup_name(name).and_then(|e| self.get_entity(e).ok())
    }
}

impl<'w> RegistryWorldExt for DeferredWorld<'w> {
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
            &*EMPTY_SET
        }
    }

    fn entity_named(&self, name: impl Into<Ustr>) -> Option<EntityRef> {
        self.lookup_name(name).and_then(|e| self.get_entity(e).ok())
    }
}

pub trait RegistryEntityWorldMutExt {
    fn entity_mut_named(&mut self, name: impl Into<Ustr>) -> Option<EntityWorldMut>;
}

impl RegistryEntityWorldMutExt for World {
    fn entity_mut_named(&mut self, name: impl Into<Ustr>) -> Option<EntityWorldMut> {
        self.lookup_name(name)
            .and_then(|e| self.get_entity_mut(e).ok())
    }
}

pub trait RegistryEntityMutExt {
    fn entity_mut_named(&mut self, name: impl Into<Ustr>) -> Option<EntityMut>;
}

impl<'w> RegistryEntityMutExt for DeferredWorld<'w> {
    fn entity_mut_named(&mut self, name: impl Into<Ustr>) -> Option<EntityMut> {
        self.lookup_name(name)
            .and_then(|e| self.get_entity_mut(e).ok())
    }
}
