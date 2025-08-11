use bevy::prelude::*;

// Entities/components
#[derive(Component, Default)]
pub struct Player;

#[derive(Component, Deref, DerefMut, Default)]
pub struct Velocity(pub Vec2);

#[derive(Component)]
pub struct Ground;

#[derive(Component)]
pub struct Exit {
    pub next: String,
    pub size: Vec2,
}

#[derive(Component)]
pub struct LevelEntity; // marker to cleanup when switching levels

#[derive(Component, Default)]
pub struct JumpState {
    pub jumping: bool,
    pub hold_ms: f32,
    pub jumps_used: u8,
}

// UI markers
#[derive(Component)]
pub struct LivesUi; // container for hearts

#[derive(Component)]
pub struct HeartSlot(pub usize);

#[derive(Component)]
pub struct GameOverUi;
