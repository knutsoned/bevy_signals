use std::{ any::TypeId, marker::PhantomData };

use bevy::{ ecs::world::Command, prelude::* };

use crate::{ framework::*, lazy_immutable::{ LazySignalsState, LazySignalsImmutable } };

/// Convenience extension to use each Command directly from Commands instance.
pub trait LazySignalsCommandsExt {
    /// Command to create a computed memo (LazyImmutable plus Propagator) from the given entity.
    fn create_computed<P: LazySignalsParams, R: LazySignalsData>(
        &mut self,
        computed: Entity,
        function: Box<dyn PropagatorContext>,
        sources: Vec<Entity>
    );

    /// Command to create an effect (Effect with no LazyImmutable) from the given entity.
    fn create_effect<P: LazySignalsParams>(
        &mut self,
        effect: Entity,
        function: Box<dyn EffectContext>,
        sources: Vec<Entity>,
        triggers: Vec<Entity>
    );

    /// Command to create a state (LazyImmutable with no Effect or Propagator) from the given entity.
    fn create_state<T: LazySignalsData>(&mut self, state: Entity, data: T);

    // Command to send a signal if the data value is different from the current value.
    fn send_signal<T: LazySignalsData>(&mut self, signal: Entity, data: T);

    // Command to send a signal even if the data value is unchanged.
    fn trigger_signal<T: LazySignalsData>(&mut self, signal: Entity, data: T);
}

impl<'w, 's> LazySignalsCommandsExt for Commands<'w, 's> {
    fn create_computed<P: LazySignalsParams, R: LazySignalsData>(
        &mut self,
        computed: Entity,
        function: Box<dyn PropagatorContext>,
        sources: Vec<Entity>
    ) {
        self.add(CreateComputedCommand::<P, R> {
            computed,
            function,
            sources,
            params_type: PhantomData,
            result_type: PhantomData,
        });
    }

    fn create_effect<P: LazySignalsParams>(
        &mut self,
        effect: Entity,
        function: Box<dyn EffectContext>,
        sources: Vec<Entity>,
        triggers: Vec<Entity>
    ) {
        self.add(CreateEffectCommand::<P> {
            effect,
            function,
            sources,
            triggers,
            params_type: PhantomData,
        });
    }

    fn create_state<T: LazySignalsData>(&mut self, state: Entity, data: T) {
        self.add(CreateStateCommand {
            state,
            data,
        });
    }

    fn send_signal<T: LazySignalsData>(&mut self, signal: Entity, data: T) {
        self.add(SendSignalCommand {
            signal,
            data,
        });
    }

    fn trigger_signal<T: LazySignalsData>(&mut self, signal: Entity, data: T) {
        self.add(TriggerSignalCommand {
            signal,
            data,
        });
    }
}

/// Command to create a computed memo (Immutable plus Propagator) from the given entity.
pub struct CreateComputedCommand<P: LazySignalsParams, R: LazySignalsData> {
    computed: Entity,
    function: Box<dyn PropagatorContext>,
    sources: Vec<Entity>,
    params_type: PhantomData<P>,
    result_type: PhantomData<R>,
}

impl<P: LazySignalsParams, R: LazySignalsData> Command for CreateComputedCommand<P, R> {
    fn apply(self, world: &mut World) {
        // once init runs once for a concrete R, it just returns the existing ComponentId next time
        let component_id = world.init_component::<LazySignalsState<R>>();
        world
            .get_entity_mut(self.computed)
            .unwrap()
            .insert((
                LazySignalsState::<R>::new(None),
                ImmutableState {
                    component_id,
                },
                ComputedImmutable {
                    function: self.function,
                    sources: self.sources,
                    params_type: TypeId::of::<P>(),
                    result_type: TypeId::of::<LazySignalsState<R>>(),
                },
                RebuildSubscribers,
            ));
    }
}

/// Command to create an effect (Propagator with no memo) from the given entity.
pub struct CreateEffectCommand<P: LazySignalsParams> {
    effect: Entity,
    function: Box<dyn EffectContext>,
    sources: Vec<Entity>,
    triggers: Vec<Entity>,
    params_type: PhantomData<P>,
}

impl<P: LazySignalsParams> Command for CreateEffectCommand<P> {
    fn apply(self, world: &mut World) {
        world
            .get_entity_mut(self.effect)
            .unwrap()
            .insert((
                LazyEffect {
                    function: self.function,
                    sources: self.sources,
                    triggers: self.triggers,
                    params_type: TypeId::of::<P>(),
                },
                RebuildSubscribers,
            ));
    }
}

/// Command to create a state (LazyImmutableImmutable) from the given entity.
pub struct CreateStateCommand<T: LazySignalsData> {
    state: Entity,
    data: T,
}

impl<T: LazySignalsData> Command for CreateStateCommand<T> {
    fn apply(self, world: &mut World) {
        // store the ComponentId so we can reflect the LazyImmutable later
        let component_id = world.init_component::<LazySignalsState<T>>();
        world
            .get_entity_mut(self.state)
            .unwrap()
            .insert((
                LazySignalsState::<T>::new(Some(Ok(self.data))),
                ImmutableState { component_id },
            ));
    }
}

/// Command to send a Signal (i.e. update a LazyImmutable during the next tick) to the given entity.
pub struct SendSignalCommand<T: LazySignalsData> {
    signal: Entity,
    data: T,
}

impl<T: LazySignalsData> Command for SendSignalCommand<T> {
    fn apply(self, world: &mut World) {
        trace!("SendSignalCommand {:?}", self.signal);
        // we're less sure the signal actually exists, but don't panic if not
        // (assume the caller removed it and we don't care about it anymore)
        if let Some(mut entity) = world.get_entity_mut(self.signal) {
            if let Some(mut immutable) = entity.get_mut::<LazySignalsState<T>>() {
                immutable.merge_next(Some(Ok(self.data)), false);
                entity.insert(SendSignal);
                trace!("merged next and inserted SendSignal");
            } else {
                error!("could not get Immutable");
            }
        } else {
            error!("could not get Signal");
        }
    }
}

/// Command to trigger a Signal (i.e. send signal even if value unchanged) to the given entity.
pub struct TriggerSignalCommand<T: LazySignalsData> {
    signal: Entity,
    data: T,
}

impl<T: LazySignalsData> Command for TriggerSignalCommand<T> {
    fn apply(self, world: &mut World) {
        trace!("TriggerSignalCommand {:?}", self.signal);
        // we're less sure the signal actually exists, but don't panic if not
        // (assume the caller removed it and we don't care about it anymore)
        if let Some(mut entity) = world.get_entity_mut(self.signal) {
            if let Some(mut immutable) = entity.get_mut::<LazySignalsState<T>>() {
                immutable.merge_next(Some(Ok(self.data)), true);
                entity.insert(SendSignal);
                trace!("merged next and inserted SendSignal");
            } else {
                error!("could not get Immutable");
            }
        } else {
            error!("could not get Signal");
        }
    }
}
