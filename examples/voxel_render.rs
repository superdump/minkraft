use bevy::{
    core::Byteable,
    input::system::exit_on_esc_system,
    prelude::*,
    reflect::TypeUuid,
    render::{
        camera::PerspectiveProjection,
        mesh::Indices,
        pipeline::{PipelineDescriptor, PipelineSpecialization, PrimitiveTopology, RenderPipeline},
        render_graph::{base, RenderGraph, RenderResourcesNode},
        renderer::{RenderResource, RenderResources},
        shader::{ShaderStage, ShaderStages},
        texture::{AddressMode, SamplerDescriptor},
    },
};
use bevy_prototype_character_controller::{
    controller::{BodyTag, CameraTag, CharacterController, HeadTag, YawTag},
    look::{LookDirection, LookEntity},
    rapier::RapierDynamicImpulseCharacterControllerPlugin,
};
use bevy_rapier3d::{
    physics::{PhysicsInterpolationComponent, RapierConfiguration, RapierPhysicsPlugin},
    rapier::{
        dynamics::{RigidBodyBuilder, RigidBodyHandle, RigidBodySet},
        geometry::{ColliderBuilder, ColliderHandle, ColliderSet},
    },
};
use minkraft::{app_state::AppState, mesh_fade::FadeUniform};
use noise::{MultiFractal, NoiseFn, RidgedMulti, Seedable};

struct Loading(Handle<Texture>);

const VERTEX_SHADER: &str = include_str!("../assets/shaders/voxel.vert");
const FRAGMENT_SHADER: &str = include_str!("../assets/shaders/voxel.frag");

#[derive(Debug, RenderResource)]
pub struct VoxelData {
    pub position: Vec4,
    pub center_to_edge: f32,
    pub texture_layer: u32,
}

unsafe impl Byteable for VoxelData {}

impl Default for VoxelData {
    fn default() -> Self {
        Self {
            position: Vec4::ZERO,
            center_to_edge: 0.5f32,
            texture_layer: 0,
        }
    }
}

#[derive(Debug, RenderResources, TypeUuid)]
#[uuid = "9bb80bb5-1f08-4e32-a342-a7e86b79c5ed"]
pub struct VoxelMap {
    #[render_resources(buffer)]
    pub voxels: Vec<VoxelData>,
}

pub struct VoxelPhysics {
    pub body_handle: RigidBodyHandle,
    pub collider_handles: Vec<ColliderHandle>,
}

const CUBE_BACKFACE_OPTIMIZATION: bool = true;
const NUM_CUBE_INDICES: usize = if CUBE_BACKFACE_OPTIMIZATION {
    3 * 3 * 2
} else {
    3 * 6 * 2
};
const NUM_CUBE_VERTICES: usize = 8;
const NUM_CUBES_PER_ROW: usize = 1414;
const NUM_CUBES: usize = NUM_CUBES_PER_ROW * NUM_CUBES_PER_ROW;
const NUM_COLLIDERS_PER_ROW: usize = 200;
const NUM_COLLIDERS: usize = NUM_COLLIDERS_PER_ROW * NUM_COLLIDERS_PER_ROW;

fn generate_index_buffer_data(num_cubes: usize) -> Vec<u32> {
    #[rustfmt::skip]
    let cube_indices = [
        // from x+, y+, z+
        1u32, 0, 2, 2, 3, 1, // back
        0, 1, 5, 5, 4, 0, // bottom
        0, 4, 6, 6, 2, 0, // left
        6, 4, 5, 5, 7, 6, // front; if not CUBE_BACKFACE_OPTIMIZATION
        7, 3, 2, 2, 6, 7, // top; if not CUBE_BACKFACE_OPTIMIZATION
        7, 5, 1, 1, 3, 7, // right; if not CUBE_BACKFACE_OPTIMIZATION
    ];

    let num_indices = num_cubes * NUM_CUBE_INDICES;

    (0..num_indices)
        .map(|i| {
            let cube = i / NUM_CUBE_INDICES;
            let cube_local = i % NUM_CUBE_INDICES;
            cube as u32 * NUM_CUBE_VERTICES as u32 + cube_indices[cube_local]
        })
        .collect()
}

/// This example illustrates how to add a custom attribute to a mesh and use it in a custom shader.
fn main() {
    env_logger::init();

    let mut app_builder = App::build();
    app_builder
        .add_plugins(DefaultPlugins)
        .insert_resource(RapierConfiguration {
            time_dependent_number_of_timesteps: true,
            ..Default::default()
        })
        // Physics - Rapier
        .add_plugin(RapierPhysicsPlugin)
        // Character Controller
        .add_plugin(RapierDynamicImpulseCharacterControllerPlugin)
        .add_system(exit_on_esc_system.system())
        // States
        .insert_resource(State::new(AppState::Loading))
        .add_state(AppState::Loading)
        // Voxel Render
        .add_system_set(SystemSet::on_enter(AppState::Loading).with_system(load_assets.system()))
        .add_system_set(SystemSet::on_update(AppState::Loading).with_system(check_loaded.system()))
        .add_system_set(
            SystemSet::on_enter(AppState::Running).with_system(setup.system().label("setup")),
        )
        .add_system_set(
            SystemSet::on_enter(AppState::Running)
                .with_system(setup_player.system().after("setup")),
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

fn setup(
    mut commands: Commands,
    texture_handle: Res<Loading>,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_graph: ResMut<RenderGraph>,
    mut bodies: ResMut<RigidBodySet>,
    mut colliders: ResMut<ColliderSet>,
) {
    render_graph.add_system_node("voxel_map", RenderResourcesNode::<VoxelMap>::new(false));
    render_graph
        .add_node_edge("voxel_map", base::node::MAIN_PASS)
        .unwrap();

    let mut texture = textures.get_mut(&texture_handle.0).unwrap();
    // Set the texture to tile over the entire quad
    texture.sampler = SamplerDescriptor {
        address_mode_u: AddressMode::Repeat,
        address_mode_v: AddressMode::Repeat,
        ..Default::default()
    };
    texture.reinterpret_stacked_2d_as_array(6);
    let material = materials.add(texture_handle.0.clone().into());

    render_graph.add_system_node(
        "fade_uniform",
        RenderResourcesNode::<FadeUniform>::new(true),
    );
    render_graph
        .add_node_edge("fade_uniform", base::node::MAIN_PASS)
        .expect("Failed to add fade_uniform as dependency of main pass");

    // Create a mesh of only indices
    let indices = generate_index_buffer_data(NUM_CUBES as usize);
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(Indices::U32(indices)));

    // Generate voxel positions and colors
    let voxel_displacement = Vec4::new(
        NUM_CUBES_PER_ROW as f32 * 0.5,
        0.0,
        NUM_CUBES_PER_ROW as f32 * 0.5,
        0.0,
    );
    let noise = RidgedMulti::new()
        .set_seed(1234)
        .set_frequency(0.008)
        .set_octaves(5);
    let mut voxels = Vec::with_capacity(NUM_CUBES as usize);

    let body_handle = bodies.insert(RigidBodyBuilder::new_static().build());
    let mut collider_handles = Vec::with_capacity(NUM_COLLIDERS);
    let colliders_lower_bound = NUM_CUBES_PER_ROW / 2 - NUM_COLLIDERS_PER_ROW / 2;
    let colliders_upper_bound = colliders_lower_bound + NUM_COLLIDERS_PER_ROW;
    for z in 0..NUM_CUBES_PER_ROW {
        for x in 0..NUM_CUBES_PER_ROW {
            let y = noise.get([x as f64, z as f64]);
            let position = Vec4::new(x as f32, 20.0 * y as f32, z as f32, 1.0) - voxel_displacement;
            if colliders_lower_bound <= x
                && x < colliders_upper_bound
                && colliders_lower_bound <= z
                && z < colliders_upper_bound
            {
                collider_handles.push(
                    colliders.insert(
                        ColliderBuilder::cuboid(0.5, 0.5, 0.5)
                            .translation(position.x, position.y, position.z)
                            .build(),
                        body_handle,
                        &mut bodies,
                    ),
                );
            }
            voxels.push(VoxelData {
                position,
                center_to_edge: 0.5f32,
                texture_layer: match y {
                    y if y < -0.5 => 1, // Blue
                    y if y < -0.4 => 2, // Yellow
                    y if y < -0.2 => 3, // Green
                    y if y < -0.1 => 4, // Brown
                    y if y < 0.6 => 5,  // Grey
                    _ => 6,             // White
                },
            });
        }
    }

    commands.insert_resource(VoxelPhysics {
        body_handle,
        collider_handles,
    });

    let voxel_map = VoxelMap { voxels };

    // Create a new shader pipeline
    let mut pipeline = PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    });
    pipeline.primitive.cull_mode = bevy::render::pipeline::CullMode::None;
    let pipeline_handle = pipelines.add(pipeline);

    // Setup our world
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(mesh),
            material,
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::specialized(
                pipeline_handle,
                PipelineSpecialization::default(),
            )]),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ..Default::default()
        })
        .insert_bundle((
            FadeUniform {
                duration: 1.0,
                remaining: 1.0,
                delay: 0.0,
                fade_in: true,
            },
            voxel_map,
        ));
}

pub struct PlayerTag;

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let spawn_pos = Vec3::new(1.1, 50.0, 1.1);
    let obj_scale = Vec3::new(0.465, 1.75, 0.25);

    let eye = Vec3::new(0.0, 4.0, 8.0);
    let center = Vec3::ZERO;
    let camera_transform = Mat4::face_toward(eye, center, Vec3::Y);

    let red = materials.add(Color::hex("DC143C").unwrap().into());
    let cuboid = meshes.add(Mesh::from(shape::Cube { size: 0.5 }));

    let head_scale = 0.3;

    let body = commands
        .spawn_bundle((
            GlobalTransform::identity(),
            Transform::from_translation(spawn_pos),
            RigidBodyBuilder::new_dynamic()
                .translation(spawn_pos.x, spawn_pos.y, spawn_pos.z)
                .lock_rotations(),
            ColliderBuilder::capsule_y(0.5 * obj_scale.y, 0.5 * obj_scale.x.max(obj_scale.z))
                .density(200.0),
            PhysicsInterpolationComponent::new(spawn_pos, Quat::IDENTITY),
            CharacterController::default(),
            BodyTag,
            PlayerTag,
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
                far: 10000.0,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert_bundle((LookDirection::default(), CameraTag))
        .id();
    commands
        .entity(body)
        .insert(LookEntity(camera))
        .push_children(&[yaw]);
    commands.entity(yaw).push_children(&[body_model, head]);
    commands.entity(head).push_children(&[head_model, camera]);

    commands.spawn_bundle(LightBundle {
        transform: Transform::from_xyz(0.0, 512.0, 0.0),
        light: Light {
            // color: Color::hex("FFA734").unwrap(),
            color: Color::ANTIQUE_WHITE,
            intensity: 1000000.0,
            depth: 0.1..1000000.0,
            range: 1000000.0,
            ..Default::default()
        },
        ..Default::default()
    });
}
