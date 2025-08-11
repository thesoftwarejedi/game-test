use bevy::prelude::*;
use serde::Deserialize;
use std::fs;
use std::path::Path;

use crate::components::{Exit, Ground, LevelEntity};
use crate::resources::{LevelManager, LevelRequest, LevelStart, PendingStart};

#[derive(Deserialize)]
struct LevelMeta { name: String }

#[derive(Deserialize)]
struct StartDef { x: f32, y: f32 }

#[derive(Deserialize)]
struct PlatformDef { x: f32, y: f32, w: f32, h: f32 }

#[derive(Deserialize)]
struct ExitDef { x: f32, y: f32, w: f32, h: f32, next: String }

#[derive(Deserialize)]
struct LevelDef {
    meta: LevelMeta,
    start: StartDef,
    #[serde(default)]
    platforms: Vec<PlatformDef>,
    #[serde(default)]
    exits: Vec<ExitDef>,
}

pub fn do_load_level(commands: &mut Commands, pending: &mut ResMut<PendingStart>, level_name: &str) {
    if let Some(def) = read_level(level_name) {
        for p in def.platforms {
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::srgb(0.20, 0.8, 0.25),
                        custom_size: Some(Vec2::new(p.w, p.h)),
                        ..default()
                    },
                    transform: Transform::from_xyz(p.x, p.y, 0.0),
                    ..default()
                },
                Ground,
                LevelEntity,
            ));
        }
        for e in def.exits {
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::srgba(0.2, 0.4, 1.0, 0.3),
                        custom_size: Some(Vec2::new(e.w, e.h)),
                        ..default()
                    },
                    transform: Transform::from_xyz(e.x, e.y, 0.5),
                    ..default()
                },
                Exit { next: e.next, size: Vec2::new(e.w, e.h) },
                LevelEntity,
            ));
        }
        let start = Vec2::new(def.start.x, def.start.y);
        pending.0 = Some(start);
        commands.insert_resource(LevelStart(start));
    }
}

fn read_level(name: &str) -> Option<LevelDef> {
    let path = format!("levels/{}.toml", name);
    let path = Path::new(&path);
    let content = fs::read_to_string(path).ok()?;
    toml::from_str::<LevelDef>(&content).ok()
}

pub fn exit_detection_system(
    mut level_req: ResMut<LevelRequest>,
    q_player: Query<&Transform, With<crate::components::Player>>,
    q_exits: Query<(&Transform, &Exit)>,
) {
    if level_req.0.is_some() { return; }
    if let Ok(pt) = q_player.get_single() {
        let p_half = crate::resources::PLAYER_SIZE / 2.0;
        let px = pt.translation.x;
        let py = pt.translation.y;
        for (et, exit) in q_exits.iter() {
            let gx = et.translation.x;
            let gy = et.translation.y;
            let half = exit.size / 2.0;
            let dx = (px - gx).abs();
            let dy = (py - gy).abs();
            if dx < (p_half.x + half.x) && dy < (p_half.y + half.y) {
                level_req.0 = Some(exit.next.clone());
                break;
            }
        }
    }
}

pub fn level_transition_system(
    mut commands: Commands,
    mut req: ResMut<LevelRequest>,
    mut pending: ResMut<PendingStart>,
    mut level_mgr: ResMut<LevelManager>,
    q_level_entities: Query<Entity, With<LevelEntity>>,
) {
    if let Some(next) = req.0.take() {
        for e in q_level_entities.iter() {
            commands.entity(e).despawn_recursive();
        }
        level_mgr.current = next.clone();
        do_load_level(&mut commands, &mut pending, &next);
    }
}
