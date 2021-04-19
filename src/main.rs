use bevy::{
    input::{keyboard::KeyCode, system::exit_on_esc_system},
    prelude::*,
    render::camera::{Camera, PerspectiveProjection},
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
    rapier::{
        dynamics::{JointSet, RigidBodySet},
        geometry::{ColliderBuilder, ColliderSet},
    },
};
use building_blocks::core::prelude::*;
use minkraft::{
    chunk_generator::{chunk_detection_system, chunk_generator_system, ChunkCommandQueue},
    debug::{Debug, DebugPlugin, DebugTransformTag},
    level_of_detail::{level_of_detail_system, LodState},
    mesh_generator::{
        mesh_generator_system, ChunkMeshes, MeshCommand, MeshCommandQueue, MeshMaterials,
    },
    voxel_map::{generate_map, NoiseConfig, VoxelMap, VoxelMapConfig},
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
        .insert_resource(NoiseConfig::default())
        .insert_resource(VoxelMapConfig::default())
        .insert_resource(ChunkCommandQueue::default())
        .add_system(
            voxel_map_config_update_system
                .system()
                .label("voxel_map_config_update"),
        )
        .add_system(
            voxel_map_config_changed_system
                .system()
                .label("voxel_map_config_changed")
                .after("voxel_map_config_update"),
        )
        .add_system(
            chunk_detection_system
                .system()
                .label("chunk_detection")
                .after("voxel_map_config_changed"),
        )
        .add_system(
            chunk_generator_system
                .system()
                .label("chunk_generator")
                .after("chunk_detection"),
        )
        .insert_resource(MeshCommandQueue::default())
        .add_system(
            level_of_detail_system
                .system()
                .label("level_of_detail")
                .after("chunk_generator"),
        )
        .add_system(
            mesh_generator_system
                .system()
                .label("mesh_generator")
                .after("level_of_detail"),
        )
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
            ColliderBuilder::capsule_y(
                0.5 * (obj_scale.y - obj_scale.x.max(obj_scale.z)),
                0.5 * obj_scale.x.max(obj_scale.z),
            )
            .density(200.0),
            PhysicsInterpolationComponent::new(spawn_pos, Quat::IDENTITY),
            CharacterController {
                run_speed: 40.0f32,
                ..Default::default()
            },
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
            perspective_projection: PerspectiveProjection {
                far: 5000.0f32,
                ..Default::default()
            },
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
    noise_config: Res<NoiseConfig>,
    voxel_map_config: Res<VoxelMapConfig>,
    mesh_commands: ResMut<MeshCommandQueue>,
) {
    let init_lod0_center =
        Point3f::from(Vec3::new(1.1, 90.0, 1.1)).in_voxel() >> voxel_map_config.chunk_log2;

    let map = init_map(
        &pool,
        &voxel_map_config,
        &noise_config,
        mesh_commands,
        init_lod0_center,
    );

    commands.insert_resource(LodState::new(init_lod0_center));
    commands.insert_resource(map);
    commands.insert_resource(ChunkMeshes::default());

    let colors = [
        Color::RED,
        Color::ORANGE,
        Color::YELLOW,
        Color::GREEN,
        Color::BLUE,
        Color::INDIGO,
        Color::VIOLET,
    ];
    let mut mesh_materials = MeshMaterials::default();
    for color in &colors {
        let mut material = StandardMaterial::from(*color);
        material.roughness = 0.9;
        mesh_materials.mesh_materials.push(materials.add(material));
    }
    commands.insert_resource(mesh_materials);

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

fn voxel_map_config_update_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut voxel_map_config: ResMut<VoxelMapConfig>,
) {
    if keyboard_input.just_pressed(KeyCode::C) {
        voxel_map_config.clip_box_radius <<= 1;
        if voxel_map_config.clip_box_radius > 128 {
            voxel_map_config.clip_box_radius = 1;
        }
    }
    if keyboard_input.just_pressed(KeyCode::L) {
        voxel_map_config.chunk_log2 += 1;
        if voxel_map_config.chunk_log2 > 7 {
            voxel_map_config.chunk_log2 = 1;
        }
        *voxel_map_config = VoxelMapConfig::new(
            voxel_map_config.chunk_log2,
            voxel_map_config.num_lods,
            voxel_map_config.clip_box_radius,
        );
    }
}

fn voxel_map_config_changed_system(
    cameras: Query<(&Camera, &GlobalTransform), With<CameraTag>>,
    pool: Res<ComputeTaskPool>,
    mut voxel_map: ResMut<VoxelMap>,
    voxel_map_config: Res<VoxelMapConfig>,
    mut lod_state: ResMut<LodState>,
    noise_config: Res<NoiseConfig>,
    mut chunk_meshes: ResMut<ChunkMeshes>,
    mut mesh_commands: ResMut<MeshCommandQueue>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut bodies: ResMut<RigidBodySet>,
    mut colliders: ResMut<ColliderSet>,
    mut joints: ResMut<JointSet>,
) {
    if !voxel_map_config.is_changed() {
        return;
    }

    chunk_meshes.clear_entities(
        &mut commands,
        &mut meshes,
        &mut bodies,
        &mut colliders,
        &mut joints,
    );
    mesh_commands.clear();

    let mut camera_position = if let Some((_camera, tfm)) = cameras.iter().next() {
        tfm.translation
    } else {
        return;
    };
    // TODO: Remove this when no longer debugging
    camera_position.y = 0.0f32;

    let lod0_center = Point3f::from(camera_position).in_voxel() >> voxel_map_config.chunk_log2;

    *voxel_map = init_map(
        &pool,
        &voxel_map_config,
        &noise_config,
        mesh_commands,
        lod0_center,
    );
    lod_state.old_lod0_center = lod0_center;
}

fn init_map(
    pool: &Res<ComputeTaskPool>,
    voxel_map_config: &Res<VoxelMapConfig>,
    noise_config: &Res<NoiseConfig>,
    mut mesh_commands: ResMut<MeshCommandQueue>,
    lod0_center: Point3i,
) -> VoxelMap {
    // Generate a voxel map from noise.
    let map = generate_map(
        pool,
        voxel_map_config.world_chunks_extent,
        noise_config,
        voxel_map_config,
    );

    // Queue up commands to initialize the chunk meshes to their appropriate LODs given the starting camera position.
    map.index.active_clipmap_lod_chunks(
        &voxel_map_config.world_voxel_extent,
        voxel_map_config.clip_box_radius,
        lod0_center,
        |chunk_key| mesh_commands.enqueue(MeshCommand::Create(chunk_key)),
    );
    assert!(!mesh_commands.is_empty());
    map
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
