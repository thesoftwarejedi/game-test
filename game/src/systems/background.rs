use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::sprite::MaterialMesh2dBundle;

// Marker for background entities
#[derive(Component)]
pub struct Background;

#[derive(Component)]
pub struct ParallaxLayer {
    // 0.0 = follows camera fully (foreground-ish), smaller moves slower; typical far layers < 1
    pub factor: f32,
    pub base_y: f32,
    pub base_x: f32,
}

pub fn setup_parallax_background(
    mut commands: Commands,
    windows: Query<&Window>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let window = windows.single();
    let width = window.width().max(1280.0);

    // Make layers very wide to avoid tiling for now
    let very_wide = width * 6.0;

    // Sky backdrop as a big rectangle behind everything
    commands.spawn((
        SpriteBundle {
            sprite: Sprite { color: Color::srgb(0.65, 0.85, 1.0), custom_size: Some(Vec2::new(very_wide, window.height() * 3.0)), ..default() },
            transform: Transform::from_xyz(0.0, 0.0, -20.0),
            ..default()
        },
        Background,
        ParallaxLayer { factor: 0.05, base_y: 0.0, base_x: 0.0 },
    ));

    // Helper to spawn a wavy hill mesh
    let mut spawn_wave = |color: Color, z: f32, factor: f32, base_y: f32, amp: f32, freq: f32, phase: f32, bottom_y: f32| {
        let segments = 120usize;
        let half_w = very_wide * 0.5;
        let dx = (very_wide) / segments as f32;

        let mut positions: Vec<[f32; 3]> = Vec::with_capacity((segments + 1) * 2);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity((segments + 1) * 2);
        for i in 0..=segments {
            let x = -half_w + i as f32 * dx;
            let y_top = base_y + amp * (freq * x + phase).sin();
            positions.push([x, y_top, z]);
            positions.push([x, bottom_y, z]);
            uvs.push([i as f32 / segments as f32, 1.0]);
            uvs.push([i as f32 / segments as f32, 0.0]);
        }

        let mut indices: Vec<u32> = Vec::with_capacity(segments * 6);
        for i in 0..segments as u32 {
            let top_left = i * 2;
            let bottom_left = top_left + 1;
            let top_right = top_left + 2;
            let bottom_right = top_left + 3;
            // two triangles per quad strip
            indices.extend_from_slice(&[top_left, bottom_left, top_right, bottom_left, bottom_right, top_right]);
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_indices(Indices::U32(indices));

        let mesh_handle = meshes.add(mesh);
        let mat_handle = materials.add(ColorMaterial::from(color));

        commands.spawn((
            MaterialMesh2dBundle {
                mesh: mesh_handle.into(),
                material: mat_handle,
                transform: Transform::from_translation(Vec3::new(0.0, 0.0, z)),
                ..default()
            },
            Background,
            ParallaxLayer { factor, base_y, base_x: 0.0 },
        ));
    };

    // Far hills
    spawn_wave(Color::srgb(0.55, 0.75, 0.85), -15.0, 0.15, 20.0, 24.0, 0.0235, 0.0, -500.0);
    // Mid hills
    spawn_wave(Color::srgb(0.40, 0.70, 0.55), -12.0, 0.30, -30.0, 36.0, 0.0095, 1.2, -500.0);
    // Near hills
    spawn_wave(Color::srgb(0.20, 0.55, 0.35), -9.0, 0.45, -120.0, 46.0, 0.0050, 2.4, -500.0);

    // Sun: slow parallax, start on the right
    // Build a simple disc mesh (triangle fan)
    let sun_segments = 64usize;
    let sun_r = 60.0f32;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(sun_segments + 2);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(sun_segments + 2);
    positions.push([0.0, 0.0, -16.0]);
    uvs.push([0.5, 0.5]);
    for i in 0..=sun_segments {
        let a = i as f32 / sun_segments as f32 * std::f32::consts::TAU;
        positions.push([a.cos() * sun_r, a.sin() * sun_r, -16.0]);
        uvs.push([a.cos() * 0.5 + 0.5, a.sin() * 0.5 + 0.5]);
    }
    let mut indices: Vec<u32> = Vec::with_capacity(sun_segments * 3);
    for i in 1..=sun_segments as u32 {
        indices.extend_from_slice(&[0, i, i + 1]);
    }
    let mut sun_mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    sun_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    sun_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    sun_mesh.insert_indices(Indices::U32(indices));
    let sun_mesh_h = meshes.add(sun_mesh);
    let sun_mat_h = materials.add(ColorMaterial::from(Color::srgb(1.0, 0.92, 0.35)));
    let sun_start_x = width * 0.6; // start on the right
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: sun_mesh_h.into(),
            material: sun_mat_h,
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, -16.0)),
            ..default()
        },
        Background,
        ParallaxLayer { factor: 0.02, base_y: 120.0, base_x: sun_start_x },
    ));
}

pub fn update_parallax_background(
    cam_q: Query<&Transform, (With<Camera>, Without<Background>)>,
    mut layers: Query<(&ParallaxLayer, &mut Transform), With<Background>>,
) {
    let cam_t = if let Ok(t) = cam_q.get_single() { t } else { return; };
    let cam_x = cam_t.translation.x;

    for (layer, mut t) in layers.iter_mut() {
        // Move opposite relative to camera to create parallax illusion.
        // factor closer to 0 -> slower movement relative to camera.
        t.translation.x = layer.base_x + cam_x * (1.0 - layer.factor);
        t.translation.y = layer.base_y; // keep vertical anchor stable
    }
}
