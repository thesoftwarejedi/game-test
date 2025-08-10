use bevy::prelude::*;

mod components;
mod resources;
mod config;
mod systems;

use config::load_config;
use resources::{GameState, LevelManager, LevelRequest, Lives, PendingStart};

fn main() {
    let cfg = load_config();

    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Rust Side Scroller".to_string(),
                    resolution: (960.0, 540.0).into(),
                    present_mode: bevy::window::PresentMode::AutoVsync,
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(cfg)
        .insert_resource(PendingStart::default())
        .insert_resource(LevelManager { current: "level1".to_string() })
        .insert_resource(LevelRequest::default())
        .insert_resource(Lives { current: 3, max: 3 })
        .insert_resource(GameState::Running)
        .add_event::<systems::particles::JumpBurstEvent>()
        .add_event::<systems::particles::DirtKickEvent>()
        .add_systems(Startup, systems::startup::setup)
        .add_systems(Update, (
            systems::player::player_input_system,
            systems::player::physics_and_collision_system,
            systems::camera::camera_follow_system,
            systems::ui::game_over_restart_system,
            systems::levels::exit_detection_system,
            systems::levels::level_transition_system,
            systems::player::apply_pending_start_system,
            systems::player::death_check_system,
            systems::ui::update_lives_ui_system,
            systems::particles::spawn_burst_on_event,
            systems::particles::spawn_dirt_on_event,
            systems::particles::update_particles,
        ))
        .run();
}
