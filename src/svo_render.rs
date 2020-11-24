use bevy::{
    core::Byteable,
    input::system::exit_on_esc_system,
    prelude::*,
    render::{
        camera::Camera,
        mesh::Indices,
        pipeline::{PipelineDescriptor, PipelineSpecialization, PrimitiveTopology, RenderPipeline},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph, RenderResourcesNode},
        renderer::{RenderResource, RenderResources},
        shader::{ShaderStage, ShaderStages},
    },
    type_registry::TypeUuid,
};
use bevy_prototype_character_controller::{
    controller::{BodyTag, CameraTag, CharacterController, HeadTag, YawTag},
    look::{LookDirection, LookEntity},
    rapier::RapierDynamicImpulseCharacterControllerPlugin,
};
use bevy_rapier3d::{
    physics::{PhysicsInterpolationComponent, RapierPhysicsPlugin},
    rapier::{
        dynamics::{RigidBodyBuilder, RigidBodyHandle, RigidBodySet},
        geometry::{ColliderBuilder, ColliderHandle, ColliderSet},
    },
};
use building_blocks::{
    core::{Extent3i, PointN},
    partition::{ChildDescriptor, ESVO},
    prelude::Array3,
    prelude::{Array, GetMut, IsEmpty},
};
use noise::{MultiFractal, NoiseFn, RidgedMulti, Seedable};

const VERTEX_SHADER: &str = r#"
#version 450

layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};

layout (set = 1, binding = 0) uniform VoxelUBO {
    vec4 VoxelUBO_camera_position;
    vec4 VoxelUBO_center_to_edge;
};

layout (set = 1, binding = 1) uniform SVOUBO {
    vec4 SVOUBO_minimum;
    vec4 SVOUBO_shape;
};

layout(set = 1, binding = 2) buffer SVOBufferStorage_children {
    uint[] children;
};

layout (location = 0) out vec3 uvw;
layout (location = 1) out vec3 local_camera_pos;
layout (location = 2) out vec3 local_pos;
layout (location = 3) out vec3 v_color;
layout (location = 4) out float v_discard;

// ** from https://fgiesen.wordpress.com/2009/12/13/decoding-morton-codes/ **

// Inverse of Part1By1 - "delete" all odd-indexed bits
uint Compact1By1(in uint x)
{
  x &= 0x55555555;                  // x = -f-e -d-c -b-a -9-8 -7-6 -5-4 -3-2 -1-0
  x = (x ^ (x >>  1)) & 0x33333333; // x = --fe --dc --ba --98 --76 --54 --32 --10
  x = (x ^ (x >>  2)) & 0x0f0f0f0f; // x = ---- fedc ---- ba98 ---- 7654 ---- 3210
  x = (x ^ (x >>  4)) & 0x00ff00ff; // x = ---- ---- fedc ba98 ---- ---- 7654 3210
  x = (x ^ (x >>  8)) & 0x0000ffff; // x = ---- ---- ---- ---- fedc ba98 7654 3210
  return x;
}

// Inverse of Part1By2 - "delete" all bits not at positions divisible by 3
uint Compact1By2(in uint x)
{
  x &= 0x09249249;                  // x = ---- 9--8 --7- -6-- 5--4 --3- -2-- 1--0
  x = (x ^ (x >>  2)) & 0x030c30c3; // x = ---- --98 ---- 76-- --54 ---- 32-- --10
  x = (x ^ (x >>  4)) & 0x0300f00f; // x = ---- --98 ---- ---- 7654 ---- ---- 3210
  x = (x ^ (x >>  8)) & 0xff0000ff; // x = ---- --98 ---- ---- ---- ---- 7654 3210
  x = (x ^ (x >> 16)) & 0x000003ff; // x = ---- ---- ---- ---- ---- --98 7654 3210
  return x;
}

uint DecodeMorton3X(in uint code)
{
  return Compact1By2(code >> 0);
}

uint DecodeMorton3Y(in uint code)
{
  return Compact1By2(code >> 1);
}

uint DecodeMorton3Z(in uint code)
{
  return Compact1By2(code >> 2);
}

// ** end **


bool is_leaf(uint child_descriptor, uint index) {
    return (child_descriptor & (1 << index)) != 0;
}

bool has_child(uint child_descriptor, uint index) {
    return (child_descriptor & (1 << (8 + index))) != 0;
}

int get_child_offset(uint child_descriptor) {
    return int(child_descriptor >> 16);
}

int get_octant_offset(uint child_descriptor, uint octant_index) {
    int octant_offset = 0;
    for (int i = 0; i < octant_index; i++) {
        if (!is_leaf(child_descriptor, uint(i)) && has_child(child_descriptor, uint(i))) {
            octant_offset++;
        }
    }
    return octant_offset;
}


const int MAX_STACK_DEPTH = 32;
const uint INVALID_OCTANT = 9;

vec3 minima[MAX_STACK_DEPTH];
float edge_lengths[MAX_STACK_DEPTH];
uint octant_indices[MAX_STACK_DEPTH];
int svo_indices[MAX_STACK_DEPTH];

void stack_push(inout int stack_index, in vec3 minimum, in float edge_length, in uint octant_index, in int svo_index) {
    stack_index++;
    minima[stack_index] = minimum;
    edge_lengths[stack_index] = edge_length;
    octant_indices[stack_index] = octant_index;
    svo_indices[stack_index] = svo_index;
}

void stack_peek(in int stack_index, out vec3 minimum, out float edge_length, out uint octant_index, out int svo_index) {
    minimum = minima[stack_index];
    edge_length = edge_lengths[stack_index];
    octant_index = octant_indices[stack_index];
    svo_index = svo_indices[stack_index];
}


void vertex_index_to_position(in uint vertex_index) {
    uvec3 xyz = uvec3(vertex_index & 0x1, (vertex_index & 0x2) >> 1, (vertex_index & 0x4) >> 2);

    if (local_camera_pos.x > 0) xyz.x = 1 - xyz.x;
    if (local_camera_pos.y > 0) xyz.y = 1 - xyz.y;
    if (local_camera_pos.z > 0) xyz.z = 1 - xyz.z;

    uvw = vec3(xyz);
    vec3 pos = uvw * 2.0 - 1.0;

    local_pos = pos.xyz * VoxelUBO_center_to_edge.xyz;
}

bool instance_to_voxel(in uint instance, inout vec3 minimum, inout float edge_length, inout int leaf_index, inout int svo_index) {
    int stack_index = -1;

    uint octant_index = INVALID_OCTANT;
    stack_push(stack_index, minimum, edge_length, octant_index, svo_index);
    while (stack_index >= 0) {
        stack_peek(stack_index, minimum, edge_length, octant_index, svo_index);
    
        if (edge_length == 1) {
            stack_index--;
            continue;
        }

        if (octant_index == INVALID_OCTANT) {
            octant_index = 0;
        } else {
            octant_index++;
        }

        uint child_descriptor = children[svo_index];
        while (octant_index < 8 && !has_child(child_descriptor, octant_index)) {
            octant_index++;
        }
        if (octant_index >= 8) {
            stack_index--;
            continue;
        }

        int child_index = svo_index
            + get_child_offset(child_descriptor)
            + get_octant_offset(child_descriptor, octant_index);

        float half_edge_length = 0.5 * edge_length;

        if (has_child(child_descriptor, octant_index)) {
            vec3 new_minimum = minimum + half_edge_length * vec3(
                DecodeMorton3X(octant_index),
                DecodeMorton3Y(octant_index),
                DecodeMorton3Z(octant_index)
            );
            octant_indices[stack_index] = octant_index;
            if (is_leaf(child_descriptor, octant_index)) {
                leaf_index++;
                if (instance == leaf_index) {
                    minimum = new_minimum;
                    edge_length = half_edge_length;
                    return true;
                }
                continue;
            }
            octant_index = INVALID_OCTANT;
            stack_push(stack_index, new_minimum, half_edge_length, octant_index, child_index);
            continue;
        }
        // pop
        stack_index--;
    }

    return false;
}

void main() {
    uint vertex_index = gl_VertexIndex;
    // 8 vertices per cube so >> 3 to get instance index
    uint instance = vertex_index >> 3;

    vec3 minimum = SVOUBO_minimum.xyz;
    float edge_length = SVOUBO_shape.x;
    int leaf_index = -1;
    int svo_index = 0;
    if (instance_to_voxel(instance, minimum, edge_length, leaf_index, svo_index)) {
        v_discard = 0.0;

        vec3 instance_position = vec3(minimum) + 0.5f * vec3(edge_length);
    
        local_camera_pos = VoxelUBO_camera_position.xyz - instance_position;
    
        vertex_index_to_position(vertex_index);
        local_pos *= edge_length;
    
        v_color = vec3(1.0, 1.0, 1.0);
    
        gl_Position = ViewProj * vec4(instance_position + local_pos, 1.0);
    } else {
        v_discard = 1.0;
    }
}
"#;

const FRAGMENT_SHADER: &str = r#"
#version 450

layout (location = 0) in vec3 uvw;
layout (location = 1) in vec3 local_camera_pos;
layout (location = 2) in vec3 local_pos;
layout (location = 3) in vec3 v_color;
layout (location = 4) in float v_discard;

layout (location = 0) out vec4 o_Target;

void main() {
    if (v_discard != 0.0) {
        discard;
    }
    o_Target = vec4(v_color, 0.5);
}
"#;

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
            camera_position: Vec3::zero().extend(1.0),
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

#[derive(Debug, RenderResources, RenderResource, TypeUuid)]
#[uuid = "9bb80bb5-1f08-4e32-a342-a7e86b79c5ed"]
#[render_resources(from_self)]
pub struct SVOUBO {
    pub minimum: Vec4,
    pub shape: Vec4,
}

unsafe impl Byteable for SVOUBO {}

#[derive(Debug, RenderResources, RenderResource)]
pub struct SVOBufferStorage {
    #[render_resources(buffer)]
    pub children: Vec<u32>,
}

unsafe impl Byteable for SVOBufferStorage {}

pub struct VoxelPhysics {
    pub body_handle: RigidBodyHandle,
    pub collider_handles: Vec<ColliderHandle>,
}

#[derive(Clone)]
pub struct Voxel(bool);

impl IsEmpty for Voxel {
    fn is_empty(&self) -> bool {
        !self.0
    }
}

const CUBE_BACKFACE_OPTIMIZATION: bool = true;
const NUM_CUBE_INDICES: usize = if CUBE_BACKFACE_OPTIMIZATION {
    3 * 3 * 2
} else {
    3 * 6 * 2
};
const NUM_CUBE_VERTICES: usize = 8;
const NUM_CUBES_PER_ROW: usize = 16;
const NUM_CUBES: usize = NUM_CUBES_PER_ROW * NUM_CUBES_PER_ROW;
const NUM_COLLIDERS_PER_ROW: usize = 16;
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
        .add_asset::<VoxelUBO>()
        .add_asset::<SVOUBO>()
        .add_startup_system(setup.system())
        // Physics - Rapier
        .add_plugin(RapierPhysicsPlugin)
        // Character Controller
        .add_plugin(RapierDynamicImpulseCharacterControllerPlugin)
        .add_system_to_stage(
            bevy::app::stage::POST_UPDATE,
            voxel_ubo_update_camera_position.system(),
        )
        .add_system(exit_on_esc_system.system());

    #[cfg(feature = "profiler")]
    app_builder.add_plugins(minkraft::diagnostics::DiagnosticPlugins);

    app_builder.run();
}

fn setup(
    commands: &mut Commands,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_graph: ResMut<RenderGraph>,
    mut voxel_ubos: ResMut<Assets<VoxelUBO>>,
    mut svo_ubos: ResMut<Assets<SVOUBO>>,
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

    render_graph.add_system_node(
        "svo_buffer_uniforms",
        AssetRenderResourcesNode::<SVOUBO>::new(false),
    );
    render_graph
        .add_node_edge("svo_buffer_uniforms", base::node::MAIN_PASS)
        .unwrap();

    render_graph.add_system_node(
        "svo_buffer_storage",
        RenderResourcesNode::<SVOBufferStorage>::new(false),
    );
    render_graph
        .add_node_edge("svo_buffer_storage", base::node::MAIN_PASS)
        .unwrap();

    // Create a new voxel uniform buffer object
    let voxel_ubo = voxel_ubos.add(VoxelUBO::default());

    // Create a mesh of only indices
    let indices = generate_index_buffer_data(NUM_CUBES as usize);
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(Indices::U32(indices)));

    let n: usize = NUM_CUBES_PER_ROW;
    let extent =
        Extent3i::from_min_and_shape(PointN([0, 0, 0]), PointN([n as i32, n as i32, n as i32]));
    let mut array = Array3::fill(extent, Voxel(false));

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

    let body_handle = bodies.insert(RigidBodyBuilder::new_static().build());
    let mut collider_handles = Vec::with_capacity(NUM_COLLIDERS);
    let colliders_lower_bound = NUM_CUBES_PER_ROW / 2 - NUM_COLLIDERS_PER_ROW / 2;
    let colliders_upper_bound = colliders_lower_bound + NUM_COLLIDERS_PER_ROW;
    for z in 0..NUM_CUBES_PER_ROW {
        for x in 0..NUM_CUBES_PER_ROW {
            // let y = noise.get([x as f64, z as f64]);
            for y in 0..NUM_CUBES_PER_ROW {
                if (x + y + z) < n {
                    let position = Vec4::new(
                        x as f32,
                        (0.5 * extent.shape.y() as f32 * (1.0 + y as f32) as f32).round(),
                        z as f32,
                        1.0,
                    ); // - voxel_displacement;
                    println!("{:?}", position);
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
                    *array.get_mut(array.stride_from_local_point(
                        &building_blocks::storage::array::Local(PointN([
                            x as i32, y as i32, z as i32,
                        ])),
                    )) = Voxel(true);
                }
            }
        }
    }

    commands.insert_resource(VoxelPhysics {
        body_handle,
        collider_handles,
    });

    let esvo = ESVO::from_array3(&array, extent);
    let svo_buffer_uniforms = svo_ubos.add(SVOUBO {
        minimum: Vec4::new(
            extent.minimum.x() as f32,
            extent.minimum.y() as f32,
            extent.minimum.z() as f32,
            0.0,
        ),
        shape: Vec4::new(
            extent.shape.x() as f32,
            extent.shape.y() as f32,
            extent.shape.z() as f32,
            0.0,
        ),
    });
    let svo_buffer_storage = SVOBufferStorage {
        children: esvo.children,
    };
    for (i, desc) in svo_buffer_storage.children.iter().enumerate() {
        println!("{}: {} {}", i, desc, <u32 as ChildDescriptor>::print(desc));
    }

    // Create a new shader pipeline
    let mut pipeline = PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    });
    pipeline
        .rasterization_state
        .as_mut()
        .map(|state| state.cull_mode = bevy::render::pipeline::CullMode::None);
    let pipeline_handle = pipelines.add(pipeline);

    // Setup our world
    commands
        .spawn(MeshBundle {
            mesh: meshes.add(mesh), // use our mesh
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::specialized(
                pipeline_handle,
                PipelineSpecialization::default(),
            )]),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ..Default::default()
        })
        .with_bundle((
            voxel_ubo.clone(),
            svo_buffer_uniforms.clone(),
            svo_buffer_storage,
        ));

    // Calling this from here to pass voxel_ubo to add to the camera
    // setup_player(commands, meshes, materials, voxel_ubo);
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_matrix(Mat4::face_toward(
                Vec3::new(32.0, 32.0, 32.0),
                Vec3::zero(),
                Vec3::unit_y(),
            )),
            ..Default::default()
        })
        .with(voxel_ubo);
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
    let center = Vec3::zero();
    let camera_transform = Mat4::face_toward(eye, center, Vec3::unit_y());

    let red = materials.add(Color::hex("DC143C").unwrap().into());
    let cuboid = meshes.add(Mesh::from(shape::Cube { size: 0.5 }));

    let head_scale = 0.3;

    let body = commands
        .spawn((
            GlobalTransform::identity(),
            Transform::from_translation(spawn_pos),
            RigidBodyBuilder::new_dynamic().translation(spawn_pos.x, spawn_pos.y, spawn_pos.z),
            ColliderBuilder::capsule_y(0.5 * obj_scale.y, 0.5 * obj_scale.x.max(obj_scale.z))
                .density(200.0),
            PhysicsInterpolationComponent::new(spawn_pos, Quat::identity()),
            CharacterController::default(),
            BodyTag,
            PlayerTag,
        ))
        .current_entity()
        .expect("Failed to spawn body");
    let yaw = commands
        .spawn((GlobalTransform::identity(), Transform::identity(), YawTag))
        .current_entity()
        .expect("Failed to spawn yaw");
    let body_model = commands
        .spawn(PbrBundle {
            material: red.clone(),
            mesh: cuboid.clone(),
            transform: Transform::from_matrix(Mat4::from_scale_rotation_translation(
                obj_scale - head_scale * Vec3::unit_y(),
                Quat::identity(),
                -0.5 * head_scale * Vec3::unit_y(),
            )),
            ..Default::default()
        })
        .current_entity()
        .expect("Failed to spawn body_model");
    let head = commands
        .spawn((
            GlobalTransform::identity(),
            Transform::from_translation(0.8 * 0.5 * obj_scale.y * Vec3::unit_y()),
            HeadTag,
        ))
        .current_entity()
        .expect("Failed to spawn head");

    let head_model = commands
        .spawn(PbrBundle {
            material: red,
            mesh: cuboid,
            transform: Transform::from_scale(Vec3::splat(head_scale)),
            ..Default::default()
        })
        .current_entity()
        .expect("Failed to spawn head_model");
    let camera = commands
        .spawn(Camera3dBundle {
            transform: Transform::from_matrix(camera_transform),
            ..Default::default()
        })
        .with_bundle((LookDirection::default(), CameraTag, voxel_ubo))
        .current_entity()
        .expect("Failed to spawn camera");
    commands
        .insert_one(body, LookEntity(camera))
        .push_children(body, &[yaw])
        .push_children(yaw, &[body_model, head])
        .push_children(head, &[head_model, camera]);
}
