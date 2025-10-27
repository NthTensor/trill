//! Defines the identity and class components, and the registry resource

mod ext;
use std::{ops::Deref, sync::LazyLock};

use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityHashMap, EntityHashSet},
    lifecycle::HookContext,
    resource::Resource,
    world::{DeferredWorld, World},
};
pub use ext::*;
use ustr::{Ustr, UstrMap};

/// Uniquely identifies an entity.
///
/// There can only be one entity with a given identiy string. Adding an identity
/// to an entity automatically removes the identity from the entity that previously held it.
#[derive(Component, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
#[component(immutable)]
#[component(on_insert = Identity::on_insert)]
#[component(on_remove = Identity::on_remove)]
pub struct Identity(Ustr);

impl Identity {
    pub fn new(str: impl Into<Ustr>) -> Identity {
        Identity(str.into())
    }

    fn on_insert(mut world: DeferredWorld, context: HookContext) {
        let Identity(name) = *world.entity(context.entity).get::<Identity>().unwrap();
        if let Some(mut registry) = world.get_resource_mut::<Registry>() {
            registry.reigrations.entry(context.entity).or_default().name = Some(name);
            if let Some(old_entity) = registry.named_entities.insert(name, context.entity) {
                registry.reigrations.get_mut(&old_entity).unwrap().name = None;
                world.commands().entity(old_entity).remove::<Identity>();
            }
        } else {
            world.commands().queue(move |world: &mut World| {
                let mut registry = world.get_resource_or_init::<Registry>();
                registry.reigrations.entry(context.entity).or_default().name = Some(name);
                if let Some(old_entity) = registry.named_entities.insert(name, context.entity) {
                    registry.reigrations.get_mut(&old_entity).unwrap().name = None;
                    world.entity_mut(old_entity).remove::<Identity>();
                }
            })
        }
    }

    fn on_remove(mut world: DeferredWorld, context: HookContext) {
        let Identity(name) = *world.entity(context.entity).get::<Identity>().unwrap();
        if let Some(mut registry) = world.get_resource_mut::<Registry>() {
            registry.reigrations.get_mut(&context.entity).unwrap().name = None;
            if registry.named_entities.get(&name) == Some(&context.entity) {
                registry.named_entities.remove(&name);
            }
        } else {
            world.commands().queue(move |world: &mut World| {
                let mut registry = world.get_resource_or_init::<Registry>();
                registry.reigrations.get_mut(&context.entity).unwrap().name = None;
                if registry.named_entities.get(&name) == Some(&context.entity) {
                    registry.named_entities.remove(&name);
                }
            });
        }
    }
}

impl Deref for Identity {
    type Target = Ustr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Identifies the class of entities to which this belongs.
///
/// Each entity may only belong to one class.
#[derive(Component, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
#[component(immutable)]
#[component(on_insert = Class::on_insert)]
#[component(on_replace = Class::on_replace)]
pub struct Class(Ustr);

impl Class {
    pub fn new(str: impl Into<Ustr>) -> Class {
        Class(str.into())
    }

    fn on_insert(mut world: DeferredWorld, context: HookContext) {
        let Class(class) = *world.entity(context.entity).get::<Class>().unwrap();
        if let Some(mut registry) = world.get_resource_mut::<Registry>() {
            registry
                .reigrations
                .entry(context.entity)
                .or_default()
                .class = Some(class);
            let class = registry.entity_classes.entry(class).or_default();
            class.insert(context.entity);
        } else {
            world.commands().queue(move |world: &mut World| {
                let mut registry = world.get_resource_or_init::<Registry>();
                registry
                    .reigrations
                    .entry(context.entity)
                    .or_default()
                    .class = Some(class);
                let class = registry.entity_classes.entry(class).or_default();
                class.insert(context.entity);
            });
        }
    }

    fn on_replace(mut world: DeferredWorld, context: HookContext) {
        let Class(class) = *world.entity(context.entity).get::<Class>().unwrap();
        if let Some(mut registry) = world.get_resource_mut::<Registry>() {
            registry.reigrations.get_mut(&context.entity).unwrap().class = None;
            let class = registry.entity_classes.entry(class).or_default();
            class.remove(&context.entity);
        } else {
            world.commands().queue(move |world: &mut World| {
                let mut registry = world.get_resource_or_init::<Registry>();
                registry.reigrations.get_mut(&context.entity).unwrap().class = None;
                let class = registry.entity_classes.entry(class).or_default();
                class.remove(&context.entity);
            });
        }
    }
}

impl Deref for Class {
    type Target = Ustr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

static EMPTY_SET: LazyLock<EntityHashSet> = LazyLock::new(EntityHashSet::default);

static EMPTY_REG: LazyLock<EntityRegistration> = LazyLock::new(EntityRegistration::default);

/// Stores mappings from names and classes to entities.
#[derive(Resource, Default)]
pub struct Registry {
    named_entities: UstrMap<Entity>,
    entity_classes: UstrMap<EntityHashSet>,
    reigrations: EntityHashMap<EntityRegistration>,
}

#[derive(Default)]
pub struct EntityRegistration {
    pub name: Option<Ustr>,
    pub class: Option<Ustr>,
}

impl Registry {
    pub fn lookup_name(&self, name: impl Into<Ustr>) -> Option<Entity> {
        self.named_entities.get(&name.into()).copied()
    }

    pub fn lookup_class(&self, class: impl Into<Ustr>) -> &EntityHashSet {
        self.entity_classes
            .get(&class.into())
            .unwrap_or(&*EMPTY_SET)
    }

    pub fn lookup_entity(&self, entity: Entity) -> &EntityRegistration {
        self.reigrations.get(&entity).unwrap_or(&*EMPTY_REG)
    }
}
