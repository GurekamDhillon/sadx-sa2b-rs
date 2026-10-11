//! Playable demo: SA2 Sonic (model, textures, physics read from your install)
//! driven by `sa_controller` on a flat plane, rendered with Bevy.
//!
//! Run: `cargo run -p sa_demo --release -- "<Sonic Adventure 2 folder>"`
//! (or set SA2_DIR). Controls: WASD / arrows move (camera-relative),
//! Space jump (press again in the air to homing-attack), Shift spin dash
//! (hold to charge, release to go), R reset. The model is shown in its rest
//! pose: animations are parsed but not applied yet.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use sa_controller::angle::{atan2_bams, to_radians};
use sa_controller::world::Ground;
use sa_controller::{Action, Input, Player, Vec3 as SVec3, World as SWorld};
use sa_data::catalog::Costume;
use sa_data::enums::Character;
use sa_data::Game;

const DEFAULT_SA2: &str = "D:/SteamLibrary/steamapps/common/Sonic Adventure 2";
const TARGETS: [[f32; 3]; 5] = [[60.0, 25.0, 0.0], [110.0, 40.0, 30.0], [150.0, 30.0, -20.0], [-60.0, 20.0, 60.0], [0.0, 35.0, -90.0]];

/// Flat floor at y = 0 plus floating homing targets.
struct DemoWorld {
    targets: Vec<SVec3>,
}

impl SWorld for DemoWorld {
    fn ground(&self, top: SVec3, bottom: SVec3) -> Option<Ground> {
        (top.y >= 0.0 && bottom.y <= 0.0).then(|| Ground { point: SVec3::new(top.x, 0.0, top.z), normal: SVec3::UP, flags: 1 })
    }

    fn homing_targets(&self, _from: SVec3) -> Vec<SVec3> {
        self.targets.clone()
    }
}

#[derive(Resource)]
struct Sim {
    player: Player,
    world: DemoWorld,
    start: Player,
    jump_pressed: bool,
    action_pressed: bool,
}

#[derive(Component)]
struct SonicRoot;
#[derive(Component)]
struct Target(usize);
#[derive(Component)]
struct Hud;
#[derive(Resource)]
struct CamYaw(f32);

fn main() {
    let dir = std::env::args().nth(1).or_else(|| std::env::var("SA2_DIR").ok()).unwrap_or_else(|| DEFAULT_SA2.into());
    let game = Game::open(&dir).unwrap_or_else(|e| panic!("open SA2 at {dir}: {e}"));
    let physics = game.physics(Character::Sonic).expect("physics");
    let player = Player::new(physics, SVec3::ZERO);
    let world = DemoWorld { targets: TARGETS.iter().map(|t| SVec3::from(*t)).collect() };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "sa-rewrite: Sonic controller demo".into(), ..default() }),
            ..default()
        }))
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(Sim { start: player.clone(), player, world, jump_pressed: false, action_pressed: false })
        .insert_resource(CamYaw(0.0))
        .insert_resource(AmbientLight { brightness: 400.0, ..default() })
        .insert_non_send_resource(game)
        .add_systems(Startup, setup)
        .add_systems(Update, (latch_buttons, sync_visuals).chain())
        .add_systems(FixedUpdate, step_player)
        .run();
}

fn rgba_image(img: &sa_formats::texture::Rgba) -> Image {
    let mut image = Image::new(
        Extent3d { width: img.width, height: img.height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        img.pixels.clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    });
    image
}

fn checker() -> Image {
    let n = 64;
    let mut px = Vec::with_capacity(n * n * 4);
    for y in 0..n {
        for x in 0..n {
            let c = if (x / 32 + y / 32) % 2 == 0 { [70, 150, 70, 255] } else { [55, 120, 55, 255] };
            px.extend_from_slice(&c);
        }
    }
    let img = sa_formats::texture::Rgba { width: n as u32, height: n as u32, pixels: px };
    rgba_image(&img)
}

fn setup(
    mut commands: Commands,
    game: NonSend<Game>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // Sonic: one Bevy mesh per textured submesh of the flattened model.
    let mesh = game.character_mesh(Character::Sonic, Costume::Default).expect("Sonic model");
    let textures = game.character_textures(Character::Sonic, Costume::Default).expect("Sonic textures");
    let tex_handles: Vec<Handle<Image>> = textures.iter().map(|(_, t)| images.add(rgba_image(t))).collect();
    let root = commands.spawn((SonicRoot, Transform::default(), Visibility::default())).id();
    for sm in &mesh.submeshes {
        let range = sm.first_index as usize..(sm.first_index + sm.index_count) as usize;
        let idx: Vec<u32> = mesh.indices[range].to_vec();
        let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, mesh.positions.clone());
        m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, mesh.normals.clone());
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, mesh.uvs.clone());
        m.insert_indices(Indices::U32(idx));
        let material = StandardMaterial {
            base_color_texture: sm.texture_id.and_then(|t| tex_handles.get(t as usize).cloned()),
            base_color: if sm.texture_id.is_some() { Color::WHITE } else { Color::srgb(0.2, 0.3, 0.9) },
            alpha_mode: AlphaMode::Mask(0.5),
            double_sided: true,
            cull_mode: None,
            perceptual_roughness: 0.6,
            ..default()
        };
        let child = commands.spawn((Mesh3d(meshes.add(m)), MeshMaterial3d(materials.add(material)), Transform::default())).id();
        commands.entity(root).add_child(child);
    }

    // Floor.
    let mut floor = Plane3d::default().mesh().size(2000.0, 2000.0).build();
    if let Some(bevy::render::mesh::VertexAttributeValues::Float32x2(uvs)) = floor.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        for uv in uvs.iter_mut() {
            uv[0] *= 50.0;
            uv[1] *= 50.0;
        }
    }
    let checker = images.add(checker());
    commands.spawn((
        Mesh3d(meshes.add(floor)),
        MeshMaterial3d(materials.add(StandardMaterial { base_color_texture: Some(checker), perceptual_roughness: 0.9, ..default() })),
    ));

    // Homing targets.
    let ball = meshes.add(Sphere::new(3.0));
    let red = materials.add(StandardMaterial { base_color: Color::srgb(0.9, 0.15, 0.1), emissive: LinearRgba::rgb(0.6, 0.05, 0.0), ..default() });
    for (i, t) in TARGETS.iter().enumerate() {
        commands.spawn((Target(i), Mesh3d(ball.clone()), MeshMaterial3d(red.clone()), Transform::from_xyz(t[0], t[1], t[2])));
    }

    commands.spawn((
        DirectionalLight { illuminance: 8000.0, shadows_enabled: true, ..default() },
        Transform::from_xyz(50.0, 100.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((Camera3d::default(), Transform::from_xyz(-40.0, 20.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y)));
    commands.spawn((
        Hud,
        Text::new(""),
        Node { position_type: PositionType::Absolute, top: Val::Px(8.0), left: Val::Px(8.0), ..default() },
    ));
}

/// Button presses can fall between fixed steps; latch them until consumed.
fn latch_buttons(keys: Res<ButtonInput<KeyCode>>, mut sim: ResMut<Sim>) {
    sim.jump_pressed |= keys.just_pressed(KeyCode::Space);
    sim.action_pressed |= keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::ShiftRight);
    if keys.just_pressed(KeyCode::KeyR) {
        sim.player = sim.start.clone();
    }
}

fn step_player(keys: Res<ButtonInput<KeyCode>>, cam: Res<CamYaw>, mut sim: ResMut<Sim>, mut targets: Query<(&Target, &mut Visibility)>) {
    let down = |a: KeyCode, b: KeyCode| keys.pressed(a) || keys.pressed(b);
    let fwd = down(KeyCode::KeyW, KeyCode::ArrowUp) as i32 as f32 - down(KeyCode::KeyS, KeyCode::ArrowDown) as i32 as f32;
    let right = down(KeyCode::KeyD, KeyCode::ArrowRight) as i32 as f32 - down(KeyCode::KeyA, KeyCode::ArrowLeft) as i32 as f32;
    let mag = (fwd * fwd + right * right).sqrt().min(1.0);
    // Forward at yaw y is (cos y, 0, sin y); screen-right is +0x4000.
    let cam_bams = (cam.0 * 65536.0 / std::f32::consts::TAU) as i32;
    let input = Input {
        stick_angle: cam_bams + atan2_bams(right, fwd),
        stick_magnitude: mag,
        jump_pressed: std::mem::take(&mut sim.jump_pressed),
        jump_held: keys.pressed(KeyCode::Space),
        action_pressed: std::mem::take(&mut sim.action_pressed),
        action_held: keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight),
    };
    let Sim { player, world, .. } = &mut *sim;
    player.step(&input, world);

    // Homing hits: bounce and pop the target; bring them all back on landing.
    if player.action == Action::Homing {
        if let Some(i) = world.targets.iter().position(|t| (*t - player.position - SVec3::new(0.0, 5.0, 0.0)).length() < 6.0) {
            player.bounce_off_target();
            world.targets[i] = SVec3::new(0.0, -10_000.0, 0.0);
            for (t, mut vis) in &mut targets {
                if t.0 == i {
                    *vis = Visibility::Hidden;
                }
            }
        }
    }
    if player.body.on_ground && world.targets.iter().any(|t| t.y < -1000.0) {
        world.targets = TARGETS.iter().map(|t| SVec3::from(*t)).collect();
        for (_, mut vis) in &mut targets {
            *vis = Visibility::Inherited;
        }
    }
}

fn sync_visuals(
    time: Res<Time>,
    sim: Res<Sim>,
    mut cam_yaw: ResMut<CamYaw>,
    mut sonic: Query<&mut Transform, (With<SonicRoot>, Without<Camera3d>)>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
    mut hud: Query<&mut Text, With<Hud>>,
) {
    let p = &sim.player;
    let pos = Vec3::new(p.position.x, p.position.y, p.position.z);
    let [rx, ry, rz] = p.body.rotation.map(to_radians);
    if let Ok(mut t) = sonic.single_mut() {
        // Same local-to-world rotation as the physics (`frame::p2g`).
        t.translation = pos;
        t.rotation = Quat::from_rotation_z(rz) * Quat::from_rotation_x(rx) * Quat::from_rotation_y(-ry);
    }

    // Chase camera: swing behind the direction of travel when moving.
    let vel = Vec3::new(p.body.velocity.x, 0.0, p.body.velocity.z);
    if vel.length() > 0.3 {
        let want = vel.z.atan2(vel.x);
        let mut d = want - cam_yaw.0;
        d = (d + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        cam_yaw.0 += d * (1.0 - (-2.0 * time.delta_secs()).exp());
    }
    let fwd = Vec3::new(cam_yaw.0.cos(), 0.0, cam_yaw.0.sin());
    if let Ok(mut c) = camera.single_mut() {
        let eye = pos - fwd * 45.0 + Vec3::Y * 18.0;
        c.translation = c.translation.lerp(eye, (1.0 - (-8.0 * time.delta_secs()).exp()).min(1.0));
        c.look_at(pos + Vec3::Y * 6.0, Vec3::Y);
    }

    if let Ok(mut text) = hud.single_mut() {
        let speed = (p.body.velocity.x.powi(2) + p.body.velocity.z.powi(2)).sqrt();
        text.0 = format!(
            "{:?}   speed {:5.2}   spin dash {:4.1}\nWASD move  Space jump / homing  Shift spin dash  R reset",
            p.action, speed, p.spindash_speed
        );
    }
}
