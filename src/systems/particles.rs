use bevy::prelude::*;

use crate::resources::PLAYER_SIZE;

// Event fired when player performs a multi-jump (>= 2nd jump)
#[derive(Event, Debug, Clone, Copy)]
pub struct JumpBurstEvent {
    pub pos: Vec2,
}

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
        // Thin ephemeral platform under feet
        let width = PLAYER_SIZE.x * 0.9;
        let height = 6.0;
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::srgb(0.95, 0.95, 1.0),
                    custom_size: Some(Vec2::new(width, height)),
                    ..default()
                },
                transform: Transform::from_xyz(ev.pos.x, ev.pos.y - PLAYER_SIZE.y * 0.5, base_z),
                ..default()
            },
            ShatterPlatform { life: 0.15 },
        ));

        // Shatter into many small particles (sand-like)
        let n = 60usize;
        let mut rng_seed = (ev.pos.x.to_bits() ^ ev.pos.y.to_bits()) as u64;
        for i in 0..n {
            // simple xorshift for determinism without rand crate
            rng_seed ^= rng_seed << 13;
            rng_seed ^= rng_seed >> 7;
            rng_seed ^= rng_seed << 17;
            let rf = |s: u32| (((((rng_seed >> s) as u32) & 0xFFFF) as f32) / 65535.0) * 2.0 - 1.0;
            let dir = Vec2::new(rf((i as u32) % 8), rf(((i as u32)+3) % 8)).normalize_or_zero();
            let speed = 140.0 + (i as f32 % 17.0) * 12.0;
            let vel = dir * speed + Vec2::new(0.0, 120.0);
            let size = 2.0 + (i % 3) as f32;
            let life = 0.6 + (i as f32 % 11.0) * 0.02;
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::srgb(0.95, 0.9, 0.75),
                        custom_size: Some(Vec2::splat(size)),
                        ..default()
                    },
                    transform: Transform::from_xyz(ev.pos.x, ev.pos.y - PLAYER_SIZE.y * 0.5, base_z + 0.01),
                    ..default()
                },
                Particle { vel, life, max_life: life },
            ));
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
