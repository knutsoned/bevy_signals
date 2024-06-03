use std::sync::RwLockReadGuard;

use bevy::{ ecs::world::World, prelude::*, reflect::TypeRegistry };

use crate::framework::*;

use super::subscribe;

fn subscribe_effect_subs(
    query_effects: &mut QueryState<(Entity, &LazyEffect), With<RebuildSubscribers>>,
    subs_closure: Box<dyn EffectSubsFn>,
    type_registry: &RwLockReadGuard<TypeRegistry>,
    world: &mut World
) {
    let mut hierarchy = EntityHierarchySet::new();

    // run the subscribe method on all Effect.sources
    for (entity, effect) in query_effects.iter(world) {
        hierarchy.insert(entity, subs_closure(effect));
    }

    for (entity, subs) in hierarchy.iter() {
        // loop through the sources
        for source in subs.iter() {
            subscribe(entity, source, type_registry, world);
        }

        // mark as processed
        world.get_entity_mut(*entity).unwrap().remove::<RebuildSubscribers>();
    }
}

fn subscribe_propagator_subs(
    query_propagators: &mut QueryState<(Entity, &ComputedImmutable), With<RebuildSubscribers>>,
    subs_closure: Box<dyn PropagatorSubsFn>,
    type_registry: &RwLockReadGuard<TypeRegistry>,
    world: &mut World
) {
    let mut hierarchy = EntityHierarchySet::new();

    // run the subscribe method on all Effect.sources
    for (entity, effect) in query_propagators.iter(world) {
        hierarchy.insert(entity, subs_closure(effect));
    }

    for (entity, subs) in hierarchy.iter() {
        // loop through the sources
        for source in subs.iter() {
            subscribe(entity, source, type_registry, world);
        }

        // mark as processed
        world.get_entity_mut(*entity).unwrap().remove::<RebuildSubscribers>();
    }
}

// FIXME should we actually just trigger everything that is marked instead of faking it?
pub fn init_effects(
    world: &mut World,
    query_effects: &mut QueryState<(Entity, &LazyEffect), With<RebuildSubscribers>>
) {
    world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
        let type_registry = type_registry.read();

        subscribe_effect_subs(
            query_effects,
            Box::new(|x: &LazyEffect| { x.sources.clone() }),
            &type_registry,
            world
        );

        subscribe_effect_subs(
            query_effects,
            Box::new(|x: &LazyEffect| { x.triggers.clone() }),
            &type_registry,
            world
        );
    });
}

// FIXME should we actually just compute everything that is marked instead of faking it?
pub fn init_memos(
    world: &mut World,
    query_propagators: &mut QueryState<(Entity, &ComputedImmutable), With<RebuildSubscribers>>
) {
    world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
        let type_registry = type_registry.read();

        subscribe_propagator_subs(
            query_propagators,
            Box::new(|x: &ComputedImmutable| { x.sources.clone() }),
            &type_registry,
            world
        );
    });
}
