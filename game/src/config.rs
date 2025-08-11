use bevy::prelude::*;
use serde::Deserialize;
use std::fs;

const DEFAULT_PLAYER_MAX_SPEED: f32 = 300.0;
const DEFAULT_ACCELERATION: f32 = 2000.0;
const DEFAULT_DECELERATION: f32 = 1800.0;
const DEFAULT_GRAVITY: f32 = 1400.0;
const DEFAULT_JUMP_VELOCITY: f32 = 600.0;
const DEFAULT_JUMP_MAX_HOLD_MS: f32 = 180.0;
const DEFAULT_JUMP_CUT_FACTOR: f32 = 0.5;

#[derive(Deserialize, Clone)]
pub struct Scalar { pub value: f32 }

#[derive(Deserialize, Clone)]
pub struct JumpCfg {
    pub velocity: f32,
    pub max_hold_ms: f32,
    pub cut_factor: f32,
    #[serde(default = "default_max_jumps")]
    pub max_jumps: u8,
}

#[derive(Deserialize, Clone)]
pub struct CameraCfg {
    pub lag_s: f32,
    pub lookahead_s: f32,
    pub noise_amp: f32,
    pub noise_freq_hz: f32,
    #[serde(default)]
    pub noise_amp_x: Option<f32>,
    #[serde(default)]
    pub noise_amp_y: Option<f32>,
}

#[derive(Deserialize, Resource, Clone)]
pub struct GameConfig {
    pub max_speed: Scalar,
    pub acceleration: Scalar,
    pub deceleration: Scalar,
    pub gravity: Scalar,
    pub jump: JumpCfg,
    pub camera: CameraCfg,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            max_speed: Scalar { value: DEFAULT_PLAYER_MAX_SPEED },
            acceleration: Scalar { value: DEFAULT_ACCELERATION },
            deceleration: Scalar { value: DEFAULT_DECELERATION },
            gravity: Scalar { value: DEFAULT_GRAVITY },
            jump: JumpCfg {
                velocity: DEFAULT_JUMP_VELOCITY,
                max_hold_ms: DEFAULT_JUMP_MAX_HOLD_MS,
                cut_factor: DEFAULT_JUMP_CUT_FACTOR,
                max_jumps: 2,
            },
            camera: CameraCfg {
                lag_s: 0.15,
                lookahead_s: 0.25,
                noise_amp: 2.0,
                noise_freq_hz: 0.7,
                noise_amp_x: None,
                noise_amp_y: None,
            },
        }
    }
}

pub fn default_max_jumps() -> u8 { 2 }

pub fn load_config() -> GameConfig {
    match fs::read_to_string("config.toml") {
        Ok(content) => toml::from_str::<GameConfig>(&content).unwrap_or_default(),
        Err(_) => GameConfig::default(),
    }
}
