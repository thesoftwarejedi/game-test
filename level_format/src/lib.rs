use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Meta {
    pub name: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub struct Start {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Rect {
    // Center-based coordinates in world space
    pub x: f32,
    pub y: f32,
    // Size (width, height)
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Exit {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub next: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Level {
    pub meta: Meta,
    pub start: Start,
    #[serde(default)]
    pub platforms: Vec<Rect>,
    #[serde(default)]
    pub exits: Vec<Exit>,
}

impl Level {
    pub fn from_toml_str(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str::<Level>(s)
    }

    pub fn to_toml_string_pretty(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}
