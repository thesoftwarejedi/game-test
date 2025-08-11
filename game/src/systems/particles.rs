use bevy::prelude::*;

use crate::resources::PLAYER_SIZE;

// Event fired when player performs a burst. kind distinguishes normal vs bonus.
#[derive(Event, Debug, Clone, Copy)]
pub struct JumpBurstEvent {
    pub pos: Vec2,
    pub kind: BurstKind,
}

// Direction-change dirt kick event
#[derive(Event, Debug, Clone, Copy)]
pub struct DirtKickEvent {
    pub pos: Vec2,
    // dir is the direction the player is starting to move (kick should fling opposite)
    pub dir: f32, // -1.0 left, 1.0 right
}

pub fn spawn_dirt_on_event(
    mut commands: Commands,
    mut reader: EventReader<DirtKickEvent>,
) {
    for ev in reader.read() {
        let base_z = 0.55;
        let n = 28usize;
        let kick_dir = -ev.dir.signum(); // fling opposite of new movement
        let mut seed = (ev.pos.x.to_bits() ^ ev.pos.y.to_bits()) as u64;
        for i in 0..n {
            // xorshift
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let rf = |s: u32| (((((seed >> s) as u32) & 0xFFFF) as f32) / 65535.0) * 2.0 - 1.0;
            let spread = 0.9; // allow farther x fling
            let vx = (kick_dir * (160.0 + (i as f32 % 9.0) * 22.0)) + rf(i as u32 % 8) * 70.0 * spread;
            let vy = 190.0 + rf(((i as u32)+3) % 8) * 60.0; // strong upward launch
            let size = 1.8 + (i % 3) as f32;
            let life = 0.34 + (i as f32 % 7.0) * 0.028; // fade before falling back down
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite { color: Color::srgb(0.35, 0.25, 0.15), custom_size: Some(Vec2::splat(size)), ..default() },
                    transform: Transform::from_xyz(ev.pos.x, ev.pos.y - PLAYER_SIZE.y * 0.5, base_z),
                    ..default()
                },
                Particle { vel: Vec2::new(vx, vy), life, max_life: life },
            ));
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BurstKind { Normal, Bonus }

// Components for transient VFX
#[derive(Component)]
pub(crate) struct ShatterPlatform {
    life: f32,
}

#[derive(Component)]
pub(crate) struct Particle {
    vel: Vec2,
    life: f32,
    max_life: f32,
}

pub fn spawn_burst_on_event(
    mut commands: Commands,
    mut reader: EventReader<JumpBurstEvent>,
) {
    for ev in reader.read() {
        let base_z = 0.6;
        // Parameters vary by kind
        let (platform_color, platform_life, n, particle_color_base, speed_base, speed_var, up_bias, sparkle_bonus) = match ev.kind {
            BurstKind::Normal => (
                Color::srgb(0.95, 0.95, 1.0),
                0.15,
                60usize,
                Color::srgb(0.95, 0.9, 0.75),
                140.0,
                12.0,
                120.0,
                false,
            ),
            BurstKind::Bonus => (
                Color::srgb(1.0, 0.95, 0.4),
                0.12,
                120usize,
                Color::srgb(1.0, 0.95, 0.6),
                200.0,
                24.0,
                180.0,
                true,
            ),
        };

        // Thin ephemeral platform under feet
        let width = PLAYER_SIZE.x * 0.95;
        let height = if matches!(ev.kind, BurstKind::Bonus) { 8.0 } else { 6.0 };
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: platform_color,
                    custom_size: Some(Vec2::new(width, height)),
                    ..default()
                },
                transform: Transform::from_xyz(ev.pos.x, ev.pos.y - PLAYER_SIZE.y * 0.5, base_z),
                ..default()
            },
            ShatterPlatform { life: platform_life },
        ));

        // Shatter into many small particles
        let mut rng_seed = (ev.pos.x.to_bits() ^ ev.pos.y.to_bits()) as u64;
        for i in 0..n {
            // simple xorshift for determinism without rand crate
            rng_seed ^= rng_seed << 13;
            rng_seed ^= rng_seed >> 7;
            rng_seed ^= rng_seed << 17;
            let rf = |s: u32| (((((rng_seed >> s) as u32) & 0xFFFF) as f32) / 65535.0) * 2.0 - 1.0;
            let dir = Vec2::new(rf((i as u32) % 8), rf(((i as u32)+3) % 8)).normalize_or_zero();
            let speed = speed_base + (i as f32 % 17.0) * speed_var;
            let vel = dir * speed + Vec2::new(0.0, up_bias);
            let size = if sparkle_bonus { 1.5 + (i % 4) as f32 } else { 2.0 + (i % 3) as f32 };
            let life = if sparkle_bonus { 0.5 + (i as f32 % 11.0) * 0.018 } else { 0.6 + (i as f32 % 11.0) * 0.02 };
            let mut color = particle_color_base;
            if sparkle_bonus {
                // randomize hue/alpha a bit for sparkly look
                let a = 0.8 + ((i % 5) as f32) * 0.04;
                color = color.with_alpha(a.min(1.0));
            }
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite { color, custom_size: Some(Vec2::splat(size)), ..default() },
                    transform: Transform::from_xyz(ev.pos.x, ev.pos.y - PLAYER_SIZE.y * 0.5, base_z + 0.01),
                    ..default()
                },
                Particle { vel, life, max_life: life },
            ));
        }

        if sparkle_bonus {
            // Extra outward ring for bonus burst
            let ring = 24usize;
            for i in 0..ring {
                let ang = (i as f32 / ring as f32) * std::f32::consts::TAU;
                let dir = Vec2::new(ang.cos(), ang.sin());
                let vel = dir * (speed_base + 90.0);
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite { color: Color::srgb(1.0, 0.8, 0.2), custom_size: Some(Vec2::splat(3.0)), ..default() },
                        transform: Transform::from_xyz(ev.pos.x, ev.pos.y - PLAYER_SIZE.y * 0.5, base_z + 0.02),
                        ..default()
                    },
                    Particle { vel, life: 0.45, max_life: 0.45 },
                ));
            }
        }
    }
}

pub fn update_particles(
    time: Res<Time>,
    cfg: Res<crate::config::GameConfig>,
    mut q_pf: Query<(Entity, &mut ShatterPlatform)>,
    mut q_p: Query<(Entity, &mut Transform, &mut Particle, &mut Sprite)>,
    mut commands: Commands,
) {
    let dt = time.delta_seconds();

    // update platform life and despawn
    for (e, mut pf) in q_pf.iter_mut() {
        pf.life -= dt;
        if pf.life <= 0.0 {
            commands.entity(e).despawn_recursive();
        }
    }

    // update particles: integrate velocity + gravity, fade out, despawn
    for (e, mut t, mut p, mut sprite) in q_p.iter_mut() {
        p.vel.y -= cfg.gravity.value * dt * 0.8; // slightly less than player gravity for feel
        t.translation.x += p.vel.x * dt;
        t.translation.y += p.vel.y * dt;
        p.life -= dt;
        let a = (p.life / p.max_life).clamp(0.0, 1.0);
        sprite.color = sprite.color.with_alpha(a);
        if p.life <= 0.0 {
            commands.entity(e).despawn_recursive();
        }
    }
}
