use bevy::prelude::*;

use crate::config::GameConfig;
use crate::resources::GROUND_Y;
use crate::components::{Player, Velocity};

pub fn camera_follow_system(
    time: Res<Time>,
    cfg: Res<GameConfig>,
    windows: Query<&Window>,
    mut q_camera: Query<&mut Transform, (With<Camera>, Without<Player>)>,
    q_player: Query<(&Transform, &Velocity), With<Player>>,
) {
    if let (Ok(mut cam_t), Ok((player_t, player_vel))) = (q_camera.get_single_mut(), q_player.get_single()) {
        let target_x = player_t.translation.x + player_vel.x * cfg.camera.lookahead_s;
        let target_y = player_t.translation.y + player_vel.y * (cfg.camera.lookahead_s * 0.6);

        let lag = cfg.camera.lag_s.max(0.0001);
        let alpha = 1.0 - (-time.delta_seconds() / lag).exp();
        cam_t.translation.x += (target_x - cam_t.translation.x) * alpha;
        cam_t.translation.y += (target_y - cam_t.translation.y) * alpha;

        let t = time.elapsed_seconds();
        let f1 = cfg.camera.noise_freq_hz.max(0.01);
        let f2 = f1 * 0.73;
        let amp_x = cfg.camera.noise_amp_x.unwrap_or(cfg.camera.noise_amp);
        let amp_y = cfg.camera.noise_amp_y.unwrap_or(cfg.camera.noise_amp);

        let window_h = windows.get_single().map(|w| w.height()).unwrap_or(540.0);
        let half_h = window_h * 0.5;
        let margin = 40.0;
        let max_cam_y = GROUND_Y + half_h - margin;
        let allowed_max = max_cam_y - amp_y;
        cam_t.translation.y = cam_t.translation.y.min(allowed_max);

        let nx = (t * std::f32::consts::TAU * f1).sin() * 0.5 * amp_x
            + (t * std::f32::consts::TAU * f2).cos() * 0.5 * amp_x;
        let ny = (t * std::f32::consts::TAU * (f1 * 0.33)).sin() * 0.5 * amp_y
            + (t * std::f32::consts::TAU * (f2 * 0.47)).cos() * 0.5 * amp_y;
        cam_t.translation.x += nx;
        cam_t.translation.y += ny;

        cam_t.translation.y = cam_t.translation.y.min(max_cam_y);
    }
}
