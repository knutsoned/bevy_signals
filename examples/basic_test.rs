use bevy::prelude::*;

use bevy_lazy_signals::{ api::LazySignals, framework::*, LazySignalsPlugin, StaticStrRef };

// simple resource to simulate a service that tracks whether a user is logged in or not
#[derive(Resource, Default)]
struct MyExampleAuthResource {
    logged_in: bool,
}
impl MyExampleAuthResource {
    fn is_logged_in(&self) -> bool {
        self.logged_in
    }
    fn notify_logged_in(&mut self) {
        self.logged_in = true;
    }
    fn notify_logged_out(&mut self) {
        self.logged_in = false;
    }
}

// this just keeps track of all the LazySignals primitives. just need the entity.
#[derive(Resource, Default)]
struct MyTestResource {
    pub computed1: Option<Entity>,
    pub computed2: Option<Entity>,
    pub effect1: Option<Entity>,
    pub effect2: Option<Entity>,
    pub effect3: Option<Entity>,
    pub signal1: Option<Entity>,
    pub signal2: Option<Entity>,
    pub signal3: Option<Entity>,
}

// concrete tuple type to safely work with the DynamicTuple coming out of the LazySignals systems
type MyClosureParams = (Option<bool>, Option<StaticStrRef>);

// making an alias to make it easier to read code in some places
type MyAuthParams = MyClosureParams;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // resource to simulate something external to update
        .init_resource::<MyExampleAuthResource>()
        // resource to hold the entity ID of each lazy signals primitive
        .init_resource::<MyTestResource>()
        // NOTE: the user application will need to register each custom LazySignalsState<T> type
        // .register_type::<LazyImmutable<MyType>>()
        // also need to register tuple types for params if they contain custom types (I think)
        // --
        // add the plugin so the signal processing systems run
        .add_plugins(LazySignalsPlugin)
        // don't need to add systems to process signals since we're using the plugin
        // just add the app-specific ones. LazySignals systems run on PreUpdate by default
        .add_systems(Startup, init)
        .add_systems(Update, send_some_signals)
        .add_systems(Last, status)
        .run();
}

fn init(mut test: ResMut<MyTestResource>, mut commands: Commands) {
    // create a signal (you need to register data types if not bool, i32, f64, or &'static str)
    // (see LazySignalsPlugin)

    // this will derive a LazySignalsImmutable<T> type based on the first parameter type
    // in this case LazySignalsImmutable<bool> is already registered so we're cool

    // in this example, signal1 would be sent whenever a user logs in or logs out
    let test_signal1 = LazySignals.state(false, &mut commands);
    test.signal1 = Some(test_signal1);
    info!("created test signal 1, entity {:#?}", test_signal1);

    // for strings the only thing I've gotten to work so far is &'static str
    let test_signal2 = LazySignals.state("Congrats, you logged in somehow", &mut commands);
    test.signal2 = Some(test_signal2);
    info!("created test signal 2, entity {:#?}", test_signal2);

    // for an effect trigger, we don't care about the value, only that it changed
    // we could use a regular signal but something like a button click might not need a type

    // there's also a way to send a regular signal as a trigger but beware: that is a good recipe
    // for an update storm
    let test_signal3 = LazySignals.state((), &mut commands);
    test.signal3 = Some(test_signal3);
    info!("created test signal 3, entity {:#?}", test_signal3);

    // simple effect that logs its trigger(s) whenever one changes
    let effect1_fn: Box<dyn Effect<MyClosureParams>> = Box::new(|params, world| {
        // read param 0
        let logged_in = params.0.unwrap();

        // read param 1
        let logged_in_msg = params.1.unwrap();

        info!("EFFECT1: got {} and {} from params", logged_in, logged_in_msg);

        // we have exclusive world access. in this case, we update a value in a resource
        world.resource_scope(|_world, mut example_auth_resource: Mut<MyExampleAuthResource>| {
            // keep our resource in sync with our signal
            if logged_in {
                example_auth_resource.notify_logged_in()
            } else {
                example_auth_resource.notify_logged_out()
            }
        });
    });

    // set up to trigger an effect from the signals
    test.effect1 = Some(
        LazySignals.effect::<MyClosureParams>(
            // closure to call when the effect is triggered
            effect1_fn,
            // type of each source must match type at same tuple position
            // it's not unsafe(?); it just won't work if we screw this up
            vec![test_signal1, test_signal2], // sending either signal triggers the effect
            // explicit triggers are not added to the params tuple like sources
            Vec::<Entity>::default(),
            &mut commands
        )
    );
    info!("created test effect 1, entity {:#?}", test.effect1.unwrap());

    // simple closure that shows a supplied value or an error message

    // this closure could be used multiple times with different entities holding the memoized value
    // and different sources
    let computed1_fn: Box<dyn Propagator<MyAuthParams, StaticStrRef>> = Box::new(|params| {
        // here we are specifically using the MyAuthParams alias to make it easier to tell what
        // these params are for, at the expense of making it easier to find the main definition

        // MyAuthParams, MyClosureParams, (Option<bool>, Option<LazySignalsStr>), and
        // (Option<bool>, Option<&str>) are interchangeable when defining propagators and effects

        // default error message (only if params.0 == false)
        let mut value = "You are not authorized to view this";

        // if loggedIn
        // (Err or None will return Err or None, this block runs only if params.0 == true)
        if params.0? {
            // show a logged in message, if one exists
            if let Some(msg) = params.1 {
                value = msg;
            } else {
                value = "Greetings, Starfighter";
            }

            // could also just do: value = params.1?;
            // and bubble the error up
        }

        info!("COMPUTED1 value: {}", value);
        Some(Ok(value))
    });

    // simple computed to store the string value or an error, depending on the bool
    let test_computed1 = LazySignals.computed::<MyAuthParams, StaticStrRef>(
        computed1_fn,
        vec![test_signal1, test_signal2], // sending either signal triggers a recompute
        &mut commands
    );
    test.computed1 = Some(test_computed1);
    info!("created test computed 1, entity {:#?}", test_computed1);

    // TODO maybe we should provide variants of Effect that take &World and no world so it isn't exclusive all the time
    let effect2_fn: Box<dyn Effect<MyClosureParams>> = Box::new(|params, _world| {
        // second effect, same as the first, but use the memo as the string instead of the signal

        // read param 0
        if let Some(logged_in) = params.0 {
            info!("EFFECT2: got logged_in: {} from params", logged_in);
        }

        // read param 1
        if let Some(logged_in_msg) = params.1 {
            info!("EFFECT2: got logged_in_msg: {} from params", logged_in_msg);
        }
    });

    // set up to trigger an effect from the memo
    test.effect2 = Some(
        LazySignals.effect::<MyClosureParams>(
            // closure to call when the effect is triggered
            effect2_fn,
            // type of each source must match type at same tuple position
            // it's not unsafe(?); it just won't work if we screw this up
            vec![test_signal1, test_computed1],
            // triggering a signal will run effects without passing the signal's value as a param
            // (it still sends the value of the sources as usual)
            vec![test_signal3],
            &mut commands
        )
    );
    info!("created test effect 2, entity {:#?}", test.effect2.unwrap());

    // this doesn't take any data, just runs when triggered
    let effect3_fn: Box<dyn Effect<()>> = Box::new(|_params, _world| {
        info!("EFFECT3: triggered");
    });

    // TODO test an effect with triggers only and no sources
    test.effect3 = Some(
        LazySignals.effect::<()>(
            // closure to call when the effect is triggered
            effect3_fn,
            // type of each source must match type at same tuple position
            // it's not unsafe(?); it just won't work if we screw this up
            Vec::<Entity>::default(),
            // triggering a signal will run effects without passing the signal's value as a param
            // (it still sends the value of the sources as usual)
            vec![test_signal3],
            &mut commands
        )
    );
    info!("created test effect 3, entity {:#?}", test.effect3.unwrap());

    let computed2_fn: Box<dyn Propagator<MyAuthParams, StaticStrRef>> = Box::new(|params| {
        // default error message
        let mut value = "You are not authorized to view this";

        // if logged_in
        if let Some(logged_in) = params.0 {
            if logged_in {
                // show a logged in message, if one exists
                if let Some(msg) = params.1 {
                    value = msg;
                }
            }
        }

        info!("COMPUTED2 value: {}", value);
        Some(Ok(value))
    });

    // simple computed to store the string value or an error, depending on the bool
    let test_computed2 = LazySignals.computed::<MyAuthParams, StaticStrRef>(
        computed2_fn,
        vec![test_signal1, test_computed1],
        &mut commands
    );
    test.computed2 = Some(test_computed2);
    info!("created test computed 2, entity {:#?}", test_computed2);

    info!("init complete");
}

fn send_some_signals(test: Res<MyTestResource>, mut commands: Commands) {
    trace!("sending 'true' to {:?}", test.signal1);
    LazySignals.send(test.signal1, true, &mut commands);

    /*
    trace!("triggering {:?}", test.signal3);
    LazySignals.trigger(test.signal3, &mut commands);
    */
}

fn status(
    world: &World,
    example_auth_resource: Res<MyExampleAuthResource>,
    test: Res<MyTestResource>
) {
    trace!("logged in: {}", example_auth_resource.is_logged_in());

    match LazySignals.read::<bool>(test.signal1, world) {
        Some(Ok(value)) => {
            trace!("value: {}", value);
        }
        Some(Err(error)) => {
            error!("error: {}", error);
        }
        None => {
            trace!("None");
        }
    }
}
