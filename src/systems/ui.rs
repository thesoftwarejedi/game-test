use bevy::prelude::*;

use crate::components::{GameOverUi, HeartSlot, LivesUi};
use crate::resources::{GameState, LevelManager, LevelRequest, LevelStart, Lives, PendingStart};

pub fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
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

    // Game Over overlay (visible even if font missing)
    let overlay = commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                visibility: Visibility::Hidden,
                ..default()
            },
            GameOverUi,
        ))
        .id();

    // Centered panel
    commands.entity(overlay).with_children(|parent| {
        parent.spawn((
            NodeBundle {
                style: Style {
                    padding: UiRect::all(Val::Px(20.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(0.2, 0.0, 0.0)),
                ..default()
            },
        ))
        .with_children(|panel| {
            // Attempt to load a font from assets; if not present, the overlay still indicates Game Over
            let font: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "GAME OVER\nPress SPACE to restart",
                    TextStyle { font, font_size: 42.0, color: Color::WHITE },
                )
                .with_justify(JustifyText::Center),
                ..default()
            });
        });
    });
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
    mut commands: Commands,
    q_level_entities: Query<Entity, With<crate::components::LevelEntity>>,
) {
    if *state != GameState::GameOver { return; }
    if keyboard.just_pressed(KeyCode::Space) {
        lives.current = lives.max;
        *state = GameState::Running;
        if let Ok(mut vis) = q_over.get_single_mut() { *vis = Visibility::Hidden; }
        // Proactively clear existing level entities to ensure a visible reset
        for e in q_level_entities.iter() {
            commands.entity(e).despawn_recursive();
        }
        level_req.0 = Some("level1".to_string());
        let start = level_start.as_ref().map(|s| s.0).unwrap_or(Vec2::ZERO);
        pending.0 = Some(start);
    }
}
