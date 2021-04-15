use bevy::{
    input::{keyboard::KeyCode, system::exit_on_esc_system},
    pbr::AmbientLight,
    prelude::*,
    tasks::ComputeTaskPool,
};
use bevy_prototype_character_controller::{
    controller::{BodyTag, CameraTag, CharacterController, HeadTag, YawTag},
    look::{LookDirection, LookEntity},
    rapier::RapierDynamicImpulseCharacterControllerPlugin,
};
use bevy_rapier3d::{
    physics::{PhysicsInterpolationComponent, RapierConfiguration, RapierPhysicsPlugin},
    rapier::dynamics::RigidBodyBuilder,
    rapier::geometry::ColliderBuilder,
};
use building_blocks::core::prelude::*;
use minkraft::{
    debug::{Debug, DebugPlugin, DebugTransformTag},
    // generate::{GeneratePlugin, GeneratedVoxelsTag},
    level_of_detail::{level_of_detail_system, LodState},
    mesh_generator::{
        mesh_generator_system, ChunkMeshes, MeshCommand, MeshCommandQueue, MeshMaterial,
    },
    voxel_map::{generate_map, CHUNK_LOG2, CLIP_BOX_RADIUS, WORLD_CHUNKS_EXTENT, WORLD_EXTENT},
    world_axes::{WorldAxes, WorldAxesCameraTag, WorldAxesPlugin},
};

fn main() {
    env_logger::builder().format_timestamp_micros().init();

    App::build()
        // Generic
        .insert_resource(WindowDescriptor {
            width: 1600.0,
            height: 900.0,
            title: env!("CARGO_PKG_NAME").to_string(),
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_system(exit_on_esc_system.system())
        // Debug
        .add_plugin(DebugPlugin)
        .add_plugin(WorldAxesPlugin)
        .add_system_to_stage(
            bevy::app::CoreStage::PreUpdate,
            toggle_debug_system.system(),
        )
        // Physics - Rapier
        .add_plugin(RapierPhysicsPlugin)
        // NOTE: This overridden configuration must come after the plugin to override the defaults
        .insert_resource(RapierConfiguration {
            time_dependent_number_of_timesteps: true,
            ..Default::default()
        })
        // Character Controller
        .add_plugin(RapierDynamicImpulseCharacterControllerPlugin)
        // Terrain
        // .add_plugin(GeneratePlugin)
        .add_system(level_of_detail_system.system())
        .add_system(mesh_generator_system.system())
        // Minkraft
        .add_startup_system(setup_world.system())
        .add_startup_system(setup_player.system())
        .run();
}

pub struct PlayerTag;

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let spawn_pos = Vec3::new(1.1, 90.0, 1.1);
    let obj_scale = Vec3::new(0.465, 1.75, 0.25);

    let eye = Vec3::new(0.0, 4.0, 8.0);
    let center = Vec3::ZERO;
    let camera_transform = Mat4::face_toward(eye, center, Vec3::Y);

    let red = materials.add(Color::hex("DC143C").unwrap().into());
    let cuboid = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));

    let head_scale = 0.3;

    let body = commands
        .spawn_bundle((
            GlobalTransform::identity(),
            Transform::from_translation(spawn_pos),
            RigidBodyBuilder::new_dynamic()
                .translation(spawn_pos.x, spawn_pos.y, spawn_pos.z)
                .lock_rotations(),
            ColliderBuilder::cuboid(0.5 * obj_scale.x, 0.5 * obj_scale.y, 0.5 * obj_scale.z)
                .density(200.0),
            PhysicsInterpolationComponent::new(spawn_pos, Quat::IDENTITY),
            CharacterController::default(),
            BodyTag,
            PlayerTag,
            // GeneratedVoxelsTag,
            DebugTransformTag,
        ))
        .id();
    let yaw = commands
        .spawn_bundle((GlobalTransform::identity(), Transform::identity(), YawTag))
        .id();
    let body_model = commands
        .spawn_bundle(PbrBundle {
            material: red.clone(),
            mesh: cuboid.clone(),
            transform: Transform::from_matrix(Mat4::from_scale_rotation_translation(
                obj_scale - head_scale * Vec3::Y,
                Quat::IDENTITY,
                -0.5 * head_scale * Vec3::Y,
            )),
            ..Default::default()
        })
        .id();
    let head = commands
        .spawn_bundle((
            GlobalTransform::identity(),
            Transform::from_translation(0.8 * 0.5 * obj_scale.y * Vec3::Y),
            HeadTag,
        ))
        .id();

    let head_model = commands
        .spawn_bundle(PbrBundle {
            material: red,
            mesh: cuboid,
            transform: Transform::from_scale(Vec3::splat(head_scale)),
            ..Default::default()
        })
        .id();
    let camera = commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_matrix(camera_transform),
            ..Default::default()
        })
        .insert_bundle((LookDirection::default(), CameraTag, WorldAxesCameraTag))
        .id();
    commands
        .entity(body)
        .insert(LookEntity(camera))
        .push_children(&[yaw]);
    commands.entity(yaw).push_children(&[body_model, head]);
    commands.entity(head).push_children(&[head_model, camera]);
}

fn setup_world(
    mut commands: Commands,
    pool: Res<ComputeTaskPool>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Generate a voxel map from noise.
    let freq = 0.15;
    let scale = 20.0;
    let seed = 666;
    let map = generate_map(&*pool, WORLD_CHUNKS_EXTENT, freq, scale, seed);

    // Queue up commands to initialize the chunk meshes to their appropriate LODs given the starting camera position.
    let init_lod0_center = Point3f::from(Vec3::new(1.1, 90.0, 1.1)).in_voxel() >> CHUNK_LOG2;
    let mut mesh_commands = MeshCommandQueue::default();
    map.index.active_clipmap_lod_chunks(
        &WORLD_EXTENT,
        CLIP_BOX_RADIUS,
        init_lod0_center,
        |chunk_key| mesh_commands.enqueue(MeshCommand::Create(chunk_key)),
    );
    assert!(!mesh_commands.is_empty());
    commands.insert_resource(mesh_commands);
    commands.insert_resource(LodState::new(init_lod0_center));
    commands.insert_resource(map);
    commands.insert_resource(ChunkMeshes::default());

    let mut material = StandardMaterial::from(Color::rgb(1.0, 0.0, 0.0));
    material.roughness = 0.9;
    commands.insert_resource(MeshMaterial(materials.add(material)));

    // commands.insert_resource(AmbientLight {
    //     color: Color::rgb_linear(1.0f32, 0.84f32, 0.67f32),
    //     brightness: 1.0 / 7.5f32,
    // });
    commands.spawn_bundle(UiCameraBundle::default());
    commands.spawn_bundle(LightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 500.0, 0.0)),
        light: Light {
            intensity: 1000000.0,
            depth: 0.1..1000000.0,
            range: 1000000.0,
            ..Default::default()
        },
        ..Default::default()
    });
}

fn toggle_debug_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut debug: ResMut<Debug>,
    mut world_axes: ResMut<WorldAxes>,
) {
    if keyboard_input.just_pressed(KeyCode::H) {
        // Use debug.enabled as the source of truth
        let new_state = !debug.enabled;
        debug.enabled = new_state;
        world_axes.enabled = new_state;
    }
}
