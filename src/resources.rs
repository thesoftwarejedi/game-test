use bevy::prelude::*;

// World constants
pub const GROUND_Y: f32 = -150.0;
pub const PLAYER_SIZE: Vec2 = Vec2::new(32.0, 32.0);

// Game-wide resources
#[derive(Resource, Clone, Copy)]
pub struct Lives { pub current: u8, pub max: u8 }

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub enum GameState { Running, GameOver }

#[derive(Resource, Default)]
pub struct LevelStart(pub Vec2);

#[derive(Resource, Default)]
pub struct PendingStart(pub Option<Vec2>);

#[derive(Resource)]
pub struct LevelManager { pub current: String }

#[derive(Resource, Default)]
pub struct LevelRequest(pub Option<String>);
