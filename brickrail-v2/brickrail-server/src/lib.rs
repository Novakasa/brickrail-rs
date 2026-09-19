use bevy::app::{Main, MainSchedulePlugin, SubApp};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use brickrail_common::layout::LayoutSubApp;

/// Server app plugin. Creates a layout SubApp with the simulation state machine.
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        let mut sub_app = SubApp::new();
        sub_app.update_schedule = Some(Main.intern());

        // Bootstrap the SubApp with the standard Bevy schedules. `SubApp::default()`
        // already schedules `message_update_system` in `First` as of bevy 0.19.
        sub_app.add_plugins(MainSchedulePlugin);

        sub_app.init_resource::<bevy::ecs::reflect::AppTypeRegistry>();

        // SimulationPlugin includes StatesPlugin, LayoutAppPlugin, and SimulationLogicPlugin.
        sub_app.add_plugins(brickrail_common::simulation::SimulationPlugin);
        sub_app.add_plugins(brickrail_common::command::SimulationCommandPlugin);
        sub_app.add_plugins(brickrail_common::command::SubAppServerPlugin);
        app.insert_sub_app(LayoutSubApp, sub_app);
    }
}
