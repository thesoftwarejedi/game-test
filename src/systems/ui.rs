use bevy::prelude::*;

use crate::components::{GameOverUi, HeartSlot, LivesUi};
use crate::resources::{GameState, LevelManager, LevelRequest, LevelStart, Lives, PendingStart};

pub fn setup_ui(mut commands: Commands) {
    // Lives hearts container
    let container = commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(8.0),
                    left: Val::Px(10.0),
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(6.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                ..default()
            },
            LivesUi,
        ))
        .id();
    for i in 0..3usize {
        commands.entity(container).with_children(|p| {
            p.spawn((
                NodeBundle {
                    style: Style { width: Val::Px(18.0), height: Val::Px(18.0), ..default() },
                    background_color: BackgroundColor(Color::srgb(0.9, 0.1, 0.2)),
                    ..default()
                },
                HeartSlot(i),
            ));
        });
    }

    // Game Over text
    commands.spawn((
        TextBundle {
            text: Text::from_section(
                "GAME OVER\nPress SPACE to restart",
                TextStyle { font_size: 42.0, color: Color::WHITE, ..default() },
            ),
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Percent(40.0),
                left: Val::Percent(20.0),
                ..default()
            },
            visibility: Visibility::Hidden,
            ..default()
        },
        GameOverUi,
    ));
}

pub fn update_lives_ui_system(
    lives: Res<Lives>,
    mut q_slots: Query<(&HeartSlot, &mut BackgroundColor)>,
) {
    if !lives.is_changed() { return; }
    for (slot, mut bg) in q_slots.iter_mut() {
        if slot.0 < lives.current as usize {
            bg.0 = Color::srgb(0.9, 0.1, 0.2);
        } else {
            bg.0 = Color::srgb(0.3, 0.3, 0.35);
        }
    }
}

pub fn game_over_restart_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut lives: ResMut<Lives>,
    mut state: ResMut<GameState>,
    mut q_over: Query<&mut Visibility, With<GameOverUi>>,
    mut pending: ResMut<PendingStart>,
    level_start: Option<Res<LevelStart>>,
    mut level_req: ResMut<LevelRequest>,
    level_mgr: Res<LevelManager>,
) {
    if *state != GameState::GameOver { return; }
    if keyboard.just_pressed(KeyCode::Space) {
        lives.current = lives.max;
        *state = GameState::Running;
        if let Ok(mut vis) = q_over.get_single_mut() { *vis = Visibility::Hidden; }
        level_req.0 = Some(level_mgr.current.clone());
        let start = level_start.as_ref().map(|s| s.0).unwrap_or(Vec2::ZERO);
        pending.0 = Some(start);
    }
}
