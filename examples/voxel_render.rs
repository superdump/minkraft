use bevy::{
    core::Byteable,
    input::system::exit_on_esc_system,
    prelude::*,
    reflect::TypeUuid,
    render::{
        camera::Camera,
        mesh::Indices,
        pipeline::{PipelineDescriptor, PipelineSpecialization, PrimitiveTopology, RenderPipeline},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph, RenderResourcesNode},
        renderer::{RenderResource, RenderResources},
        shader::{ShaderStage, ShaderStages},
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
use noise::{MultiFractal, NoiseFn, RidgedMulti, Seedable};

const VERTEX_SHADER: &str = include_str!("../assets/shaders/voxel.vert");

const FRAGMENT_SHADER: &str = include_str!("../assets/shaders/voxel.frag");

#[derive(Debug, RenderResources, RenderResource, TypeUuid)]
#[uuid = "c63fd9ae-3847-4c7e-a33d-29f2dea49501"]
#[render_resources(from_self)]
pub struct VoxelUBO {
    camera_position: Vec4,
    center_to_edge: Vec4,
}

unsafe impl Byteable for VoxelUBO {}

impl Default for VoxelUBO {
    fn default() -> Self {
        Self {
            camera_position: Vec3::ZERO.extend(1.0),
            center_to_edge: Vec4::splat(0.5),
        }
    }
}

pub fn voxel_ubo_update_camera_position(
    mut voxel_ubos: ResMut<Assets<VoxelUBO>>,
    query: Query<(&Camera, &GlobalTransform, &Handle<VoxelUBO>)>,
) {
    for (camera, transform, voxel_ubo_handle) in query.iter() {
        if let Some(name) = camera.name.as_ref() {
            if name == "Camera3d" {
                let voxel_ubo = voxel_ubos.get_mut(voxel_ubo_handle).unwrap();
                voxel_ubo.camera_position = transform.translation.extend(1.0);
            }
        }
    }
}

#[derive(Debug, RenderResource)]
pub struct VoxelData {
    pub position: Vec4,
    pub color: Vec4,
}

unsafe impl Byteable for VoxelData {}

impl Default for VoxelData {
    fn default() -> Self {
        Self {
            position: Vec4::ZERO,
            color: Vec4::ONE,
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
        .add_asset::<VoxelUBO>()
        .add_startup_system(setup.system())
        // Physics - Rapier
        .add_plugin(RapierPhysicsPlugin)
        // Character Controller
        .add_plugin(RapierDynamicImpulseCharacterControllerPlugin)
        .add_system_to_stage(
            CoreStage::PostUpdate,
            voxel_ubo_update_camera_position.system(),
        )
        .add_system(exit_on_esc_system.system());

    #[cfg(feature = "profiler")]
    app_builder.add_plugins(minkraft::diagnostics::DiagnosticPlugins);

    app_builder.run();
}

fn setup(
    mut commands: Commands,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_graph: ResMut<RenderGraph>,
    mut voxel_ubos: ResMut<Assets<VoxelUBO>>,
    mut bodies: ResMut<RigidBodySet>,
    mut colliders: ResMut<ColliderSet>,
    materials: ResMut<Assets<StandardMaterial>>,
) {
    render_graph.add_system_node(
        "voxel_ubo",
        AssetRenderResourcesNode::<VoxelUBO>::new(false),
    );
    render_graph
        .add_node_edge("voxel_ubo", base::node::MAIN_PASS)
        .unwrap();

    render_graph.add_system_node("voxel_map", RenderResourcesNode::<VoxelMap>::new(false));
    render_graph
        .add_node_edge("voxel_map", base::node::MAIN_PASS)
        .unwrap();

    // Create a new voxel uniform buffer object
    let voxel_ubo = voxel_ubos.add(VoxelUBO::default());

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
    let colors = [
        Vec4::new(0.275, 0.51, 0.706, 1.0),  // Blue
        Vec4::new(1.0, 0.98, 0.804, 1.0),    // Yellow
        Vec4::new(0.604, 0.804, 0.196, 1.0), // Green
        Vec4::new(0.545, 0.271, 0.075, 1.0), // Brown
        Vec4::new(0.502, 0.502, 0.502, 1.0), // Grey
        Vec4::new(1.0, 0.98, 0.98, 1.0),     // White
    ];

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
                color: colors[match y {
                    y if y < -0.5 => 0, // Blue
                    y if y < -0.4 => 1, // Yellow
                    y if y < -0.2 => 2, // Green
                    y if y < -0.1 => 3, // Brown
                    y if y < 0.6 => 4,  // Grey
                    _ => 5,             // White
                }],
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
        .spawn_bundle(MeshBundle {
            mesh: meshes.add(mesh), // use our mesh
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::specialized(
                pipeline_handle,
                PipelineSpecialization::default(),
            )]),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ..Default::default()
        })
        .insert_bundle((voxel_ubo.clone(), voxel_map));

    // Calling this from here to pass voxel_ubo to add to the camera
    setup_player(&mut commands, meshes, materials, voxel_ubo);
}

pub struct PlayerTag;

fn setup_player(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    voxel_ubo: Handle<VoxelUBO>,
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
            ..Default::default()
        })
        .insert_bundle((LookDirection::default(), CameraTag, voxel_ubo))
        .id();
    commands
        .entity(body)
        .insert(LookEntity(camera))
        .push_children(&[yaw]);
    commands.entity(yaw).push_children(&[body_model, head]);
    commands.entity(head).push_children(&[head_model, camera]);
}
