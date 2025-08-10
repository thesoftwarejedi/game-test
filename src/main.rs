use bevy::prelude::*;
use serde::Deserialize;
use std::fs;
use std::path::Path;

// Basic side scroller constants
// Defaults are overridden by config.toml if present
const DEFAULT_PLAYER_MAX_SPEED: f32 = 300.0; // pixels/sec
const DEFAULT_ACCELERATION: f32 = 2000.0; // pixels/sec^2
const DEFAULT_DECELERATION: f32 = 1800.0; // pixels/sec^2
const DEFAULT_GRAVITY: f32 = 1400.0; // pixels/sec^2
const DEFAULT_JUMP_VELOCITY: f32 = 600.0; // pixels/sec
const DEFAULT_JUMP_MAX_HOLD_MS: f32 = 180.0; // ms
const DEFAULT_JUMP_CUT_FACTOR: f32 = 0.5; // scale v.y on early release

// World setup
const GROUND_Y: f32 = -150.0;
const GROUND_SIZE: Vec2 = Vec2::new(1000.0, 40.0);
const PLAYER_SIZE: Vec2 = Vec2::new(32.0, 32.0);

#[derive(Component, Default)]
struct Player;

#[derive(Component, Deref, DerefMut, Default)]
struct Velocity(Vec2);

#[derive(Component)]
struct Ground;

#[derive(Component)]
struct Exit {
    next: String,
    size: Vec2,
}

// ---------- Lives, UI, and Game Over ----------
fn death_check_system(
    mut lives: ResMut<Lives>,
    mut state: ResMut<GameState>,
    mut pending: ResMut<PendingStart>,
    level_start: Option<Res<LevelStart>>,
    q_player: Query<&Transform, With<Player>>,
    mut q_over: Query<&mut Visibility, With<GameOverUi>>,
) {
    if *state == GameState::GameOver { return; }
    const DEATH_Y: f32 = -600.0;
    if let Ok(t) = q_player.get_single() {
        if t.translation.y < DEATH_Y {
            if lives.current > 0 { lives.current -= 1; }
            if lives.current == 0 {
                *state = GameState::GameOver;
                if let Ok(mut vis) = q_over.get_single_mut() { *vis = Visibility::Visible; }
            } else {
                let start = level_start.as_ref().map(|s| s.0).unwrap_or(Vec2::ZERO);
                pending.0 = Some(start);
            }
        }
    }
}

fn update_lives_ui_system(
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

fn game_over_restart_system(
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
        // Reload current level
        level_req.0 = Some(level_mgr.current.clone());
        // Ensure player will spawn at start
        let start = level_start.as_ref().map(|s| s.0).unwrap_or(Vec2::ZERO);
        pending.0 = Some(start);
    }
}

#[derive(Component)]
struct LevelEntity; // marker to cleanup when switching levels

// ---------- Lives/UI types ----------
#[derive(Resource, Clone, Copy)]
struct Lives { current: u8, max: u8 }

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
enum GameState { Running, GameOver }

#[derive(Component)]
struct LivesUi; // container for hearts

#[derive(Component)]
struct HeartSlot(pub usize);

#[derive(Component)]
struct GameOverUi;

#[derive(Resource, Default)]
struct LevelStart(pub Vec2);

#[derive(Component, Default)]
struct JumpState {
    jumping: bool,
    hold_ms: f32,
    jumps_used: u8,
}

#[derive(Deserialize, Clone)]
struct Scalar { value: f32 }

#[derive(Deserialize, Clone)]
struct JumpCfg {
    velocity: f32,
    max_hold_ms: f32,
    cut_factor: f32,
    #[serde(default = "default_max_jumps")]
    max_jumps: u8,
}

#[derive(Deserialize, Clone)]
struct CameraCfg {
    // Time constant (seconds) for smoothing toward target
    lag_s: f32,
    // How far ahead to look in the direction of player velocity (seconds)
    lookahead_s: f32,
    // Noise amplitude in pixels
    noise_amp: f32,
    // Noise base frequency in Hz
    noise_freq_hz: f32,
    // Optional per-axis amplitudes; if omitted, fall back to noise_amp
    #[serde(default)]
    noise_amp_x: Option<f32>,
    #[serde(default)]
    noise_amp_y: Option<f32>,
}

#[derive(Deserialize, Resource, Clone)]
struct GameConfig {
    max_speed: Scalar,
    acceleration: Scalar,
    deceleration: Scalar,
    gravity: Scalar,
    jump: JumpCfg,
    camera: CameraCfg,
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

fn default_max_jumps() -> u8 { 2 }

// Removed horizontal world bounds; allow free movement

#[derive(Resource, Default)]
struct PendingStart(Option<Vec2>);

#[derive(Resource)]
struct LevelManager {
    current: String,
}

#[derive(Resource, Default)]
struct LevelRequest(Option<String>);

// ---------- Level file schema ----------

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

fn main() {
    // Load config from config.toml if available
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
            // Make sprites crisp for pixel-art feel
            .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(cfg)
        .insert_resource(PendingStart::default())
        .insert_resource(LevelManager { current: "level1".to_string() })
        .insert_resource(LevelRequest::default())
        .insert_resource(Lives { current: 3, max: 3 })
        .insert_resource(GameState::Running)
        .add_systems(Startup, setup)
        .add_systems(Update, (
            player_input_system,
            physics_and_collision_system,
            camera_follow_system,
            exit_detection_system,
            level_transition_system,
            apply_pending_start_system,
            death_check_system,
            update_lives_ui_system,
            game_over_restart_system,
        ))
        .run();
}

fn load_config() -> GameConfig {
    match fs::read_to_string("config.toml") {
        Ok(content) => toml::from_str::<GameConfig>(&content).unwrap_or_default(),
        Err(_) => GameConfig::default(),
    }
}

fn setup(
    mut commands: Commands,
    level_mgr: Res<LevelManager>,
    mut pending: ResMut<PendingStart>,
) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    // UI: Lives hearts (top-left) as red squares (no font dependency)
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

    // UI: Game Over (hidden initially)
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

fn player_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    q_player: Query<(&Velocity, &Transform), With<Player>>,
) {
    if let Ok((_vel, _transform)) = q_player.get_single() {
        // Physics system reads input and applies acceleration/deceleration.
        // Keep here in case we later want to add air control modifiers etc.
        let _left = keyboard.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
        let _right = keyboard.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);
        // No direct mutation of vel.x here.
    }
}

fn physics_and_collision_system(
    time: Res<Time>,
    cfg: Res<GameConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut q_player: Query<(&mut Transform, &mut Velocity, &mut JumpState), With<Player>>,
    q_ground: Query<(&Transform, &Sprite), (With<Ground>, Without<Player>)>,
) {
    let dt = time.delta_seconds();

    if let Ok((mut t, mut v, mut jump)) = q_player.get_single_mut() {
        // Horizontal movement with accel/decel
        let mut input_dir = 0.0f32;
        if keyboard.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) { input_dir -= 1.0; }
        if keyboard.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) { input_dir += 1.0; }

        let target_speed = input_dir * cfg.max_speed.value;
        if input_dir.abs() > 0.0 {
            v.x = approach(v.x, target_speed, cfg.acceleration.value * dt);
        } else {
            v.x = approach(v.x, 0.0, cfg.deceleration.value * dt);
        }

        // Gravity
        v.y -= cfg.gravity.value * dt;

        // Integrate
        t.translation.x += v.x * dt;
        t.translation.y += v.y * dt;

        // Simple AABB collision with each ground
        let player_half = PLAYER_SIZE / 2.0;
        let mut grounded = false;
        for (gt, gsprite) in q_ground.iter() {
            let gsize = gsprite.custom_size.unwrap_or(Vec2::splat(0.0));
            let ground_half = gsize / 2.0;

            let px = t.translation.x;
            let py = t.translation.y;
            let gx = gt.translation.x;
            let gy = gt.translation.y;

            let dx = (px - gx).abs();
            let dy = (py - gy).abs();
            // penetration depths (positive if overlapping)
            let pen_x = player_half.x + ground_half.x - dx;
            let pen_y = player_half.y + ground_half.y - dy;

            // If overlapping
            if pen_x > 0.0 && pen_y > 0.0 {
                // Resolve along the minimal penetration axis
                if pen_y < pen_x {
                    // Resolve vertically
                    if py > gy {
                        // From above -> place on top, mark grounded
                        t.translation.y = gy + ground_half.y + player_half.y;
                        v.y = 0.0;
                        grounded = true;
                    } else {
                        // From below -> push down
                        t.translation.y = gy - ground_half.y - player_half.y;
                        if v.y > 0.0 { v.y = 0.0; }
                    }
                } else {
                    // Resolve horizontally
                    if px > gx {
                        t.translation.x = gx + ground_half.x + player_half.x;
                    } else {
                        t.translation.x = gx - ground_half.x - player_half.x;
                    }
                    v.x = 0.0;
                }
            }
        }

        // Jumping: allow double jump up to max_jumps
        if keyboard.just_pressed(KeyCode::Space) {
            let can_jump = if grounded { true } else { jump.jumps_used < cfg.jump.max_jumps };
            if can_jump {
                v.y = cfg.jump.velocity;
                jump.jumping = true;
                jump.hold_ms = 0.0;
                if grounded { jump.jumps_used = 1; } else { jump.jumps_used = (jump.jumps_used + 1).min(cfg.jump.max_jumps); }
            }
        }
        // Track hold time while rising
        if jump.jumping && keyboard.pressed(KeyCode::Space) && v.y > 0.0 {
            jump.hold_ms += dt * 1000.0;
        }
        // Early release jump cut
        if keyboard.just_released(KeyCode::Space) {
            if jump.hold_ms < cfg.jump.max_hold_ms && v.y > 0.0 {
                v.y *= cfg.jump.cut_factor;
            }
            jump.jumping = false;
        }
        // Reset jump state if landed
        if grounded && v.y.abs() < f32::EPSILON {
            jump.jumping = false;
            jump.hold_ms = 0.0;
            jump.jumps_used = 0;
        }

        // No horizontal clamp: allow moving beyond edges
    }
}

fn camera_follow_system(
    time: Res<Time>,
    cfg: Res<GameConfig>,
    windows: Query<&Window>,
    mut q_camera: Query<&mut Transform, (With<Camera>, Without<Player>)>,
    q_player: Query<(&Transform, &Velocity), With<Player>>,
) {
    if let (Ok(mut cam_t), Ok((player_t, player_vel))) = (q_camera.get_single_mut(), q_player.get_single()) {
        // Target with velocity lookahead
        let target_x = player_t.translation.x + player_vel.x * cfg.camera.lookahead_s;
        let target_y = player_t.translation.y + player_vel.y * (cfg.camera.lookahead_s * 0.6);

        // Exponential smoothing toward target using lag time constant
        let lag = cfg.camera.lag_s.max(0.0001);
        let alpha = 1.0 - (-time.delta_seconds() / lag).exp();
        cam_t.translation.x += (target_x - cam_t.translation.x) * alpha;
        cam_t.translation.y += (target_y - cam_t.translation.y) * alpha;

        // Subtle human-like noise (breathing/inaccuracies) on X and slight Y
        let t = time.elapsed_seconds();
        let f1 = cfg.camera.noise_freq_hz.max(0.01);
        let f2 = f1 * 0.73;
        let amp_x = cfg.camera.noise_amp_x.unwrap_or(cfg.camera.noise_amp);
        let amp_y = cfg.camera.noise_amp_y.unwrap_or(cfg.camera.noise_amp);

        // Clamp baseline Y before noise so positive phase isn't truncated as often
        let window_h = windows.get_single().map(|w| w.height()).unwrap_or(540.0);
        let half_h = window_h * 0.5;
        let margin = 40.0; // pixels, small ground margin
        let max_cam_y = GROUND_Y + half_h - margin;
        let allowed_max = max_cam_y - amp_y; // leave headroom for noise
        cam_t.translation.y = cam_t.translation.y.min(allowed_max);

        // Subtle human-like noise on X and Y with equal structure
        let nx = (t * std::f32::consts::TAU * f1).sin() * 0.5 * amp_x
            + (t * std::f32::consts::TAU * f2).cos() * 0.5 * amp_x;
        let ny = (t * std::f32::consts::TAU * (f1 * 0.33)).sin() * 0.5 * amp_y
            + (t * std::f32::consts::TAU * (f2 * 0.47)).cos() * 0.5 * amp_y;
        cam_t.translation.x += nx;
        cam_t.translation.y += ny;

        // Final safety clamp to keep the ground visible
        cam_t.translation.y = cam_t.translation.y.min(max_cam_y);
    }
}

fn approach(current: f32, target: f32, max_delta: f32) -> f32 {
    let delta = target - current;
    let step = max_delta.clamp(0.0, delta.abs());
    current + step * delta.signum()
}

// ---------- Level helpers & systems ----------

fn do_load_level(commands: &mut Commands, pending: &mut ResMut<PendingStart>, level_name: &str) {
    if let Some(def) = read_level(level_name) {
        // Spawn platforms
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

        // Spawn exits (semi-transparent blue)
        for e in def.exits {
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgba(0.2, 0.4, 1.0, 0.3),
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

        // Set player start to be applied next frame and remember for respawns
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

fn apply_pending_start_system(
    mut pending: ResMut<PendingStart>,
    mut q_player: Query<(&mut Transform, &mut Velocity, &mut JumpState), With<Player>>,
    mut q_camera: Query<&mut Transform, (With<Camera>, Without<Player>)>,
) {
    if let Some(pos) = pending.0.take() {
        if let Ok((mut t, mut v, mut j)) = q_player.get_single_mut() {
            t.translation.x = pos.x;
            t.translation.y = pos.y;
            v.0 = Vec2::ZERO;
            j.jumping = false;
            j.hold_ms = 0.0;
            j.jumps_used = 0;
        }
        if let Ok(mut cam_t) = q_camera.get_single_mut() {
            cam_t.translation.x = pos.x;
            cam_t.translation.y = 0.0; // reset baseline Y so camera doesn't stay low
        }
    }
}

fn exit_detection_system(
    mut level_req: ResMut<LevelRequest>,
    q_player: Query<&Transform, With<Player>>,
    q_exits: Query<(&Transform, &Exit)>,
) {
    if level_req.0.is_some() { return; }
    if let Ok(pt) = q_player.get_single() {
        let p_half = PLAYER_SIZE / 2.0;
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

fn level_transition_system(
    mut commands: Commands,
    mut req: ResMut<LevelRequest>,
    mut pending: ResMut<PendingStart>,
    mut level_mgr: ResMut<LevelManager>,
    q_level_entities: Query<Entity, With<LevelEntity>>,
) {
    if let Some(next) = req.0.take() {
        // Despawn previous level entities
        for e in q_level_entities.iter() {
            commands.entity(e).despawn_recursive();
        }
        level_mgr.current = next.clone();
        do_load_level(&mut commands, &mut pending, &next);
    }
}
