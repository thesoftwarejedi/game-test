use bevy::prelude::*;

use crate::components::{JumpState, Player, Velocity};
use crate::resources::{LevelManager, PendingStart, PLAYER_SIZE};
use crate::systems::levels::do_load_level;
use crate::systems::ui::setup_ui;

pub fn setup(
    mut commands: Commands,
    level_mgr: Res<LevelManager>,
    mut pending: ResMut<PendingStart>,
    asset_server: Res<AssetServer>,
) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    // UI
    setup_ui(commands.reborrow(), asset_server);

    // Player
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
                custom_size: Some(PLAYER_SIZE),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..default()
        },
        Player,
        Velocity::default(),
        JumpState::default(),
    ));

    // Load initial level
    do_load_level(&mut commands, &mut pending, &level_mgr.current);
}
