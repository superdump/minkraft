use bevy::{
    asset::AssetServerSettings,
    input::{keyboard::KeyCode, system::exit_on_esc_system},
    prelude::*,
    render::{
        camera::PerspectiveProjection,
        pipeline::{FrontFace, PipelineDescriptor, RenderPipeline},
        render_graph::{base, RenderGraph, RenderResourcesNode},
        shader::{shader_defs_system, ShaderStage, ShaderStages},
        texture::{AddressMode, SamplerDescriptor},
    },
    tasks::ComputeTaskPool,
};
use bevy_frustum_culling::*;
use bevy_mod_bounding::*;
use bevy_physical_sky::{
    PhysicalSkyCameraTag, PhysicalSkyMaterial, PhysicalSkyPlugin, SolarPosition,
    PHYSICAL_SKY_FRAGMENT_SHADER, PHYSICAL_SKY_PASS_TIME_SYSTEM, PHYSICAL_SKY_VERTEX_SHADER,
};
use bevy_prototype_character_controller::{
    controller::{BodyTag, CameraTag, CharacterController, HeadTag, YawTag},
    look::{LookDirection, LookEntity},
    rapier::RapierDynamicImpulseCharacterControllerPlugin,
};
use bevy_rapier3d::{
    physics::{PhysicsInterpolationComponent, RapierConfiguration, RapierPhysicsPlugin},
    rapier::{dynamics::RigidBodyBuilder, geometry::ColliderBuilder},
};
use building_blocks::core::prelude::*;
use minkraft::{
    app_state::AppState,
    debug::{Debug, DebugPlugin, DebugTransformTag},
    level_of_detail::LodState,
    mesh_fade::FadeUniform,
    mesh_generator::{ArrayTextureMaterial, ArrayTexturePipelines, ChunkMeshes, MeshCommandQueue},
    shaders::{ARRAY_TEXTURE_FRAGMENT_SHADER, ARRAY_TEXTURE_VERTEX_SHADER},
    voxel_map::{NoiseConfig, VoxelMap, VoxelMapConfig, VoxelMapPlugin},
    world_axes::{WorldAxes, WorldAxesCameraTag, WorldAxesPlugin},
};

struct Loading(Handle<Texture>);

const SPAWN_POINT: [f32; 3] = [8.5, 641.0, -3.5];
const NO_GRAVITY: [f32; 3] = [0.0, 0.0, 0.0];
const GRAVITY: [f32; 3] = [0.0, -9.81, 0.0];

fn main() {
    env_logger::builder().format_timestamp_micros().init();

    App::build()
        // Generic
        .insert_resource(WindowDescriptor {
            width: 1600.0,
            height: 900.0,
            title: env!("CARGO_PKG_NAME").to_string(),
            vsync: false,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .insert_resource(AssetServerSettings {
            asset_folder: env!("CARGO_MANIFEST_DIR").to_string(),
        })
        .add_system(exit_on_esc_system.system())
        // States
        .insert_resource(State::new(AppState::Loading))
        .add_state(AppState::Loading)
        // Debug
        .add_plugin(DebugPlugin)
        .add_plugin(WorldAxesPlugin)
        .add_system_to_stage(
            bevy::app::CoreStage::PreUpdate,
            toggle_debug_system.system(),
        )
        .add_system_to_stage(
            bevy::app::CoreStage::PreUpdate,
            toggle_gravity_system.system(),
        )
        // Physics - Rapier
        .add_plugin(RapierPhysicsPlugin)
        // NOTE: This overridden configuration must come after the plugin to override the defaults
        .insert_resource(RapierConfiguration {
            gravity: NO_GRAVITY.into(),
            time_dependent_number_of_timesteps: true,
            ..Default::default()
        })
        // Character Controller
        .add_plugin(RapierDynamicImpulseCharacterControllerPlugin)
        // Terrain
        // For fade in/out
        .add_system_to_stage(
            CoreStage::PostUpdate,
            shader_defs_system::<FadeUniform>.system(),
        )
        .add_plugin(VoxelMapPlugin)
        // Frustum culling
        .add_plugin(BoundingVolumePlugin::<obb::Obb>::default())
        .add_plugin(FrustumCullingPlugin::<obb::Obb>::default())
        // Minkraft
        .add_system_set(SystemSet::on_enter(AppState::Loading).with_system(load_assets.system()))
        .add_system_set(SystemSet::on_update(AppState::Loading).with_system(check_loaded.system()))
        .insert_resource(SolarPosition {
            // Stockholm
            latitude: 59.33258,
            longitude: 18.0649,
            // one day per 8 minutes of real time
            simulation_seconds_per_second: 24.0 * 60.0 * 60.0 / (8.0 * 60.0),
            ..Default::default()
        })
        .add_plugin(PhysicalSkyPlugin)
        .add_system(
            update_sun_light_position
                .system()
                .label("update_sun_light_position")
                .after(PHYSICAL_SKY_PASS_TIME_SYSTEM),
        )
        .add_system_set(SystemSet::on_enter(AppState::Running).with_system(setup_graphics.system()))
        .add_system_set(
            SystemSet::on_enter(AppState::Running)
                .with_system(setup_world.system().label("setup_world")),
        )
        .add_system_set(
            SystemSet::on_enter(AppState::Running)
                .with_system(setup_player.system().after("setup_world")),
        )
        .run();
}

fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load("textures/voxel-pack/array_texture.png");
    commands.insert_resource(Loading(handle));
}

/// Make sure that our texture is loaded so we can change some settings on it later
fn check_loaded(
    mut state: ResMut<State<AppState>>,
    handle: Res<Loading>,
    asset_server: Res<AssetServer>,
) {
    if let bevy::asset::LoadState::Loaded = asset_server.get_load_state(&handle.0) {
        state.set(AppState::Running).unwrap();
    }
}

fn setup_graphics(
    mut commands: Commands,
    texture_handle: Res<Loading>,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut sky_materials: ResMut<Assets<PhysicalSkyMaterial>>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    // Create a new shader pipeline
    let mut pipeline_descriptor = PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(
            ShaderStage::Vertex,
            PHYSICAL_SKY_VERTEX_SHADER,
        )),
        fragment: Some(shaders.add(Shader::from_glsl(
            ShaderStage::Fragment,
            PHYSICAL_SKY_FRAGMENT_SHADER,
        ))),
    });
    // Reverse the winding so we can see the faces from the inside
    pipeline_descriptor.primitive.front_face = FrontFace::Cw;
    let pipeline = pipelines.add(pipeline_descriptor);

    // Create a new material
    let material = sky_materials.add(PhysicalSkyMaterial::stellar_dawn(true));

    // Sky box cube
    commands
        .spawn_bundle(MeshBundle {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                radius: 4900.0,
                subdivisions: 5,
            })),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(pipeline)]),
            transform: Transform::from_xyz(SPAWN_POINT[0], SPAWN_POINT[1], SPAWN_POINT[2]),
            ..Default::default()
        })
        .insert(material);

    let mut texture = textures.get_mut(&texture_handle.0).unwrap();
    // Set the texture to tile over the entire quad
    texture.sampler = SamplerDescriptor {
        address_mode_u: AddressMode::Repeat,
        address_mode_v: AddressMode::Repeat,
        ..Default::default()
    };
    texture.reinterpret_stacked_2d_as_array(6);
    let mut material = StandardMaterial::from(texture_handle.0.clone());
    material.roughness = 0.6;
    let material_handle = materials.add(material);
    commands.insert_resource(ArrayTextureMaterial(material_handle));

    render_graph.add_system_node(
        "fade_uniform",
        RenderResourcesNode::<FadeUniform>::new(true),
    );
    render_graph
        .add_node_edge("fade_uniform", base::node::MAIN_PASS)
        .expect("Failed to add fade_uniform as dependency of main pass");

    let pipeline = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(
            ShaderStage::Vertex,
            ARRAY_TEXTURE_VERTEX_SHADER,
        )),
        fragment: Some(shaders.add(Shader::from_glsl(
            ShaderStage::Fragment,
            ARRAY_TEXTURE_FRAGMENT_SHADER,
        ))),
    }));

    commands.insert_resource(ArrayTexturePipelines(RenderPipelines::from_pipelines(
        vec![RenderPipeline::new(pipeline)],
    )));
}

pub struct PlayerTag;

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let spawn_pos = SPAWN_POINT.into();
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
        .insert_bundle((
            CameraTag,
            FrustumCulling,
            LookDirection::default(),
            PhysicalSkyCameraTag,
            WorldAxesCameraTag,
        ))
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
    noise_config: Res<NoiseConfig>,
    voxel_map_config: Res<VoxelMapConfig>,
    mesh_commands: ResMut<MeshCommandQueue>,
) {
    let init_lod0_center = PointN(SPAWN_POINT).in_voxel() >> voxel_map_config.chunk_log2;

    let map = VoxelMap::new(
        &pool,
        &voxel_map_config,
        &noise_config,
        mesh_commands,
        init_lod0_center,
    );

    commands.insert_resource(LodState::new(init_lod0_center));
    commands.insert_resource(map);
    commands.insert_resource(ChunkMeshes::default());

    commands.spawn_bundle(UiCameraBundle::default());
    commands.spawn_bundle(LightBundle {
        transform: Transform::from_translation(Vec3::new(
            SPAWN_POINT[0] + 1000.0,
            SPAWN_POINT[1] + 512.0,
            SPAWN_POINT[2] + 3200.0,
        )),
        light: Light {
            // color: Color::hex("FFA734").unwrap(),
            color: Color::ANTIQUE_WHITE,
            intensity: 10000000.0,
            depth: 0.1..1000000.0,
            range: 1000000.0,
            ..Default::default()
        },
        ..Default::default()
    });
}

const DEGREES_TO_RADIANS: f64 = std::f64::consts::PI / 180.0;

fn update_sun_light_position(
    solar_position: Res<SolarPosition>,
    mut query: Query<&mut Transform, With<Light>>,
) {
    let (azimuth, inclination) = solar_position.get_azimuth_inclination();
    let (azimuth_radians, inclination_radians) = (
        ((azimuth * DEGREES_TO_RADIANS) - std::f64::consts::PI) as f32,
        (inclination * DEGREES_TO_RADIANS) as f32,
    );
    let translation = Vec3::new(
        azimuth_radians.cos(),
        azimuth_radians.sin() * inclination_radians.sin(),
        azimuth_radians.sin() * inclination_radians.cos(),
    )
    .normalize()
        * 4500.0;
    for mut transform in query.iter_mut() {
        *transform = Transform::from_translation(translation);
    }
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

fn toggle_gravity_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut rapier_config: ResMut<RapierConfiguration>,
) {
    if keyboard_input.just_pressed(KeyCode::G) {
        rapier_config.gravity = if rapier_config.gravity.y == GRAVITY[1] {
            NO_GRAVITY.into()
        } else {
            GRAVITY.into()
        };
    }
}
