use bevy::prelude::*;

use crate::components::{JumpState, Player, Velocity};
use crate::config::GameConfig;
use crate::resources::{GameState, LevelStart, PendingStart, PLAYER_SIZE};
use crate::systems::particles::{JumpBurstEvent, BurstKind, DirtKickEvent};

pub fn player_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    q_player: Query<(&Velocity, &Transform), With<Player>>,
) {
    if let Ok((_vel, _transform)) = q_player.get_single() {
        let _left = keyboard.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
        let _right = keyboard.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);
    }
}

pub fn physics_and_collision_system(
    time: Res<Time>,
    cfg: Res<GameConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut q_player: Query<(&mut Transform, &mut Velocity, &mut JumpState), With<Player>>,
    q_ground: Query<(&Transform, &Sprite), (With<crate::components::Ground>, Without<Player>)>,
    mut ev_burst: EventWriter<JumpBurstEvent>,
    mut ev_dirt: EventWriter<DirtKickEvent>,
) {
    let dt = time.delta_seconds();

    if let Ok((mut t, mut v, mut jump)) = q_player.get_single_mut() {
        let mut input_dir = 0.0f32;
        if keyboard.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) { input_dir -= 1.0; }
        if keyboard.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) { input_dir += 1.0; }

        let target_speed = input_dir * cfg.max_speed.value;
        let prev_vx = v.x;
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
            let ground_half = gsprite.custom_size.unwrap_or(Vec2::ZERO) / 2.0;

            let px = t.translation.x;
            let py = t.translation.y;
            let gx = gt.translation.x;
            let gy = gt.translation.y;

            let dx = (px - gx).abs();
            let dy = (py - gy).abs();
            let pen_x = player_half.x + ground_half.x - dx;
            let pen_y = player_half.y + ground_half.y - dy;

            if pen_x > 0.0 && pen_y > 0.0 {
                // One-way platforms: ignore collisions when coming from below while moving upward
                let player_bottom = py - player_half.y;
                let ground_top = gy + ground_half.y;
                if player_bottom < ground_top && v.y > 0.0 {
                    // Allow the player to pass through from below
                    continue;
                }

                if pen_y < pen_x {
                    if py > gy {
                        // Land on top
                        t.translation.y = gy + ground_half.y + player_half.y;
                        v.y = 0.0;
                        grounded = true;
                    } else {
                        // Coming from below: do not push the player down (already handled by pass-through logic)
                        // If we ever reach here (e.g., v.y <= 0), avoid bumping head
                        continue;
                    }
                } else {
                    // Horizontal resolution
                    if px > gx {
                        t.translation.x = gx + ground_half.x + player_half.x;
                    } else {
                        t.translation.x = gx - ground_half.x - player_half.x;
                    }
                    v.x = 0.0;
                }
            }
        }

        // Dirt kick: emit when reversing direction on ground
        if grounded && input_dir.abs() > 0.0 {
            let new_dir = input_dir.signum();
            let old_dir = prev_vx.signum();
            if old_dir != 0.0 && new_dir != 0.0 && old_dir != new_dir && prev_vx.abs() > 30.0 {
                ev_dirt.send(DirtKickEvent { pos: Vec2::new(t.translation.x, t.translation.y), dir: new_dir });
            }
        }

        // Jumping: allow up to max_jumps
        if keyboard.just_pressed(KeyCode::Space) {
            let (can_jump, bonus_triggered) = if grounded { 
                (true, false) 
            } else { 
                // 10% chance to grant an extra jump: refund one usage
                // simple hash-based RNG without external crates
                let tbits = (time.elapsed_seconds() * 1_000_000.0) as u32;
                let xb = t.translation.x.to_bits();
                let yb = t.translation.y.to_bits();
                let mut h = xb ^ yb ^ tbits ^ (jump.jumps_used as u32);
                // xorshift
                h ^= h << 13;
                h ^= h >> 17;
                h ^= h << 5;
                let r01 = (h as f32 / u32::MAX as f32).clamp(0.0, 1.0);
                let bonus_triggered = if r01 < 0.10 && jump.jumps_used > 1 {
                    jump.jumps_used -= 1; // refund one, effectively adding an extra jump
                    true
                } else {
                    false
                };
                (jump.jumps_used < cfg.jump.max_jumps, bonus_triggered)
            };
            if can_jump {
                v.y = cfg.jump.velocity;
                jump.jumping = true;
                jump.hold_ms = 0.0;
                if grounded {
                    jump.jumps_used = 1;
                } else {
                    jump.jumps_used = (jump.jumps_used + 1).min(cfg.jump.max_jumps);
                }
                // Emit burst event(s)
                if bonus_triggered {
                    ev_burst.send(JumpBurstEvent { pos: Vec2::new(t.translation.x, t.translation.y), kind: BurstKind::Bonus });
                }
                if jump.jumps_used >= 2 {
                    ev_burst.send(JumpBurstEvent { pos: Vec2::new(t.translation.x, t.translation.y), kind: BurstKind::Normal });
                }
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
    }
}

#[inline]
fn approach(current: f32, target: f32, max_delta: f32) -> f32 {
    let delta = target - current;
    let step = max_delta.clamp(0.0, delta.abs());
    current + step * delta.signum()
}

pub fn death_check_system(
    mut lives: ResMut<crate::resources::Lives>,
    mut state: ResMut<GameState>,
    mut pending: ResMut<PendingStart>,
    level_start: Option<Res<LevelStart>>,
    q_player: Query<&Transform, With<Player>>,
    mut q_over: Query<&mut Visibility, With<crate::components::GameOverUi>>,
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

pub fn apply_pending_start_system(
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
            cam_t.translation.y = 0.0;
        }
    }
}
