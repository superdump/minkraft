use bevy::{
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics},
    prelude::*,
    render::{mesh::VertexAttribute, pipeline::PrimitiveTopology},
    tasks::ComputeTaskPool,
};
use bevy_rapier3d::{
    physics::{ColliderHandleComponent, RigidBodyHandleComponent},
    rapier::{
        dynamics::{JointSet, RigidBodyBuilder, RigidBodyHandle, RigidBodySet},
        geometry::{ColliderBuilder, ColliderSet},
    },
};
use building_blocks::core::prelude::*;
use building_blocks::mesh::{
    greedy_quads, pos_norm_tex_meshes_from_material_quads, GreedyQuadsBuffer, MaterialVoxel,
    PosNormTexMesh,
};
use building_blocks::storage::{prelude::*, IsEmpty};
use noise::{MultiFractal, NoiseFn, RidgedMulti, Seedable};
use std::collections::HashMap;

const SEA_LEVEL: f64 = 64.0;
const TERRAIN_Y_SCALE: f64 = 0.2;

pub const CHUNK_GENERATION_DURATION: DiagnosticId =
    DiagnosticId::from_u128(231527775571401697783537491377266602078);
pub const MESH_GENERATION_DURATION: DiagnosticId =
    DiagnosticId::from_u128(81564243874222570218257378919410104882);
pub const MESH_INDEX_COUNT: DiagnosticId =
    DiagnosticId::from_u128(118084277781716293979909451698540294716);

fn setup_diagnostic_system(mut diagnostics: ResMut<Diagnostics>) {
    // Diagnostics must be initialized before measurements can be added.
    // In general it's a good idea to set them up in a "startup system".
    diagnostics.add(Diagnostic::new(
        CHUNK_GENERATION_DURATION,
        "chunk_generation_duration",
        0,
    ));
    diagnostics.add(Diagnostic::new(
        MESH_GENERATION_DURATION,
        "mesh_generation_duration",
        0,
    ));
    diagnostics.add(Diagnostic::new(MESH_INDEX_COUNT, "mesh_index_count", 0));
}

type NoiseType = RidgedMulti;
type VoxelMap = ChunkMap3<Voxel, ()>;
type VoxelMaterial = u8;

pub struct GeneratedVoxelsTag;

struct GeneratedMeshesResource {
    pub generated_map: HashMap<Point3i, Vec<(Entity, Handle<Mesh>, RigidBodyHandle)>>,
}

impl Default for GeneratedMeshesResource {
    fn default() -> Self {
        GeneratedMeshesResource {
            generated_map: HashMap::new(),
        }
    }
}

struct GeneratedVoxelResource {
    pub chunk_size: i32,
    pub map: VoxelMap,
    pub max_height: i32,
    pub view_distance: i32,
    pub materials: Vec<Handle<StandardMaterial>>,
}

impl Default for GeneratedVoxelResource {
    fn default() -> Self {
        let chunk_size = 16;
        GeneratedVoxelResource {
            chunk_size,
            map: ChunkMap3::new(PointN([chunk_size; 3]), Voxel(0), (), FastLz4 { level: 10 }),
            max_height: 256,
            view_distance: 256,
            materials: Vec::new(),
        }
    }
}

pub struct GeneratePlugin;

impl Plugin for GeneratePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_resource::<GeneratedVoxelResource>(GeneratedVoxelResource::default())
            .add_resource::<GeneratedMeshesResource>(GeneratedMeshesResource::default())
            .add_startup_system(init_generation.system())
            .add_startup_system(setup_diagnostic_system.system())
            .add_system(generate_voxels.system())
            .add_system(generate_meshes.system());
    }
}

fn init_generation(
    mut res: ResMut<GeneratedVoxelResource>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    res.materials.push(materials.add(Color::NONE.into()));
    res.materials
        .push(materials.add(Color::hex("4682B4").unwrap().into())); // Blue
    res.materials
        .push(materials.add(Color::hex("FFFACD").unwrap().into())); // Yellow
    res.materials
        .push(materials.add(Color::hex("9ACD32").unwrap().into())); // Green
    res.materials
        .push(materials.add(Color::hex("8B4513").unwrap().into())); // Brown
    res.materials
        .push(materials.add(Color::hex("808080").unwrap().into())); // Grey
    res.materials
        .push(materials.add(Color::hex("FFFAFA").unwrap().into())); // White
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Voxel(VoxelMaterial);

impl Default for Voxel {
    fn default() -> Self {
        Voxel(0)
    }
}

impl IsEmpty for Voxel {
    fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

impl MaterialVoxel for Voxel {
    type Material = VoxelMaterial;

    fn material(&self) -> Self::Material {
        self.0
    }
}

fn height_to_material(y: i32) -> VoxelMaterial {
    match y {
        y if (y as f64) < 0.85 * SEA_LEVEL => 1, // Blue
        y if (y as f64) < 0.87 * SEA_LEVEL => 2, // Yellow
        y if (y as f64) < 0.90 * SEA_LEVEL => 3, // Green
        y if (y as f64) < 0.92 * SEA_LEVEL => 4, // Brown
        y if (y as f64) < 1.1 * SEA_LEVEL => 5,  // Grey
        _ => 6,                                  // White
    }
}

fn generate_chunk(extent: Extent3i) -> Array3<Voxel> {
    let yoffset = SEA_LEVEL;
    let yscale = TERRAIN_Y_SCALE * yoffset;
    let min = extent.minimum;
    let max = extent.least_upper_bound();

    let noise = NoiseType::new()
        .set_seed(1234)
        .set_frequency(0.008)
        .set_octaves(5);
    let heightmap: Vec<Vec<i32>> = (min.x()..max.x())
        .map(|x| {
            (min.z()..max.z())
                .map(|z| (noise.get([x as f64, z as f64]) * yscale + yoffset).round() as i32)
                .collect()
        })
        .collect();

    Array3::fill_with(extent, |p| {
        let height = heightmap[(p.x() - min.x()) as usize][(p.z() - min.z()) as usize];
        if p.y() <= height {
            Voxel(height_to_material(height))
        } else {
            Voxel(0)
        }
    })
}

fn generate_voxels(
    voxel_meshes: Res<GeneratedMeshesResource>,
    task_pool: Res<ComputeTaskPool>,
    mut voxels: ResMut<GeneratedVoxelResource>,
    mut diagnostics: ResMut<Diagnostics>,
    _cam: &GeneratedVoxelsTag,
    cam_transform: &Transform,
) {
    let cam_pos = cam_transform.translation();
    let cam_pos = PointN([cam_pos.x().round() as i32, 0i32, cam_pos.z().round() as i32]);

    let max_height = voxels.max_height;
    let extent = transform_to_extent(cam_pos, voxels.view_distance, max_height);
    let extent = extent_modulo_expand(extent, voxels.chunk_size);
    let min = extent.minimum;
    let max = extent.least_upper_bound();

    let chunk_size = voxels.chunk_size;
    let vd2 = voxels.view_distance * voxels.view_distance;

    let start = std::time::Instant::now();

    let chunks = task_pool.scope(|s| {
        for z in (min.z()..max.z()).step_by(chunk_size as usize) {
            for x in (min.x()..max.x()).step_by(chunk_size as usize) {
                let p = PointN([x, min.y(), z]);
                let d = p - cam_pos;
                if voxel_meshes.generated_map.get(&p).is_some() || d.dot(&d) > vd2 {
                    continue;
                }

                s.spawn(async move {
                    generate_chunk(Extent3i::from_min_and_shape(
                        PointN([x, min.y(), z]),
                        PointN([chunk_size, max.y(), chunk_size]),
                    ))
                })
            }
        }
    });

    for chunk in &chunks {
        copy_extent(chunk.extent(), chunk, &mut voxels.map);
    }

    if chunks.len() > 0 {
        let dur = std::time::Instant::now() - start;
        diagnostics.add_measurement(
            CHUNK_GENERATION_DURATION,
            dur.as_secs_f64() * 1000.0 / chunks.len() as f64,
        );
    }
}

fn transform_to_extent(cam_pos: Point3i, view_distance: i32, max_height: i32) -> Extent3i {
    Extent3i::from_min_and_lub(
        PointN([cam_pos.x() - view_distance, 0, cam_pos.z() - view_distance]),
        PointN([
            cam_pos.x() + view_distance,
            max_height,
            cam_pos.z() + view_distance,
        ]),
    )
}

fn modulo_down(v: i32, modulo: i32) -> i32 {
    (v / modulo) * modulo
}

fn modulo_up(v: i32, modulo: i32) -> i32 {
    ((v / modulo) + 1) * modulo
}

fn extent_modulo_expand(extent: Extent3i, modulo: i32) -> Extent3i {
    let min = extent.minimum;
    let max = extent.least_upper_bound();
    Extent3i::from_min_and_lub(
        PointN([
            modulo_down(min.x(), modulo),
            min.y(),
            modulo_down(min.z(), modulo),
        ]),
        PointN([
            modulo_up(max.x(), modulo) + 1,
            max.y() + 1,
            modulo_up(max.z(), modulo) + 1,
        ]),
    )
}

fn generate_mesh(voxel_map: &VoxelMap, extent: Extent3i) -> fnv::FnvHashMap<u8, PosNormTexMesh> {
    let padded_extent = extent.padded(1);

    let mut map = Array3::fill(padded_extent, Voxel(0));

    let local_cache = LocalChunkCache::new();
    let reader = ChunkMapReader3::new(voxel_map, &local_cache);
    copy_extent(&padded_extent, &reader, &mut map);

    let mut quads = GreedyQuadsBuffer::new(padded_extent);
    greedy_quads(&map, &padded_extent, &mut quads);

    pos_norm_tex_meshes_from_material_quads(&quads.quad_groups)
}

fn spawn_meshes(
    commands: &mut Commands,
    diagnostics: &mut ResMut<Diagnostics>,
    meshes: &mut ResMut<Assets<Mesh>>,
    mut bodies: &mut ResMut<RigidBodySet>,
    colliders: &mut ResMut<ColliderSet>,
    materials: &[Handle<StandardMaterial>],
    pos_norm_tex_ind: &fnv::FnvHashMap<u8, PosNormTexMesh>,
) -> Vec<(Entity, Handle<Mesh>, RigidBodyHandle)> {
    let mut entities = Vec::with_capacity(pos_norm_tex_ind.len());
    for (i, pos_norm_tex_ind) in pos_norm_tex_ind {
        let indices: Vec<u32> = pos_norm_tex_ind.indices.iter().map(|i| *i as u32).collect();
        let mesh = meshes.add(Mesh {
            primitive_topology: PrimitiveTopology::TriangleList,
            attributes: vec![
                VertexAttribute::position(pos_norm_tex_ind.positions.clone()),
                VertexAttribute::normal(pos_norm_tex_ind.normals.clone()),
                VertexAttribute::uv(pos_norm_tex_ind.tex_coords.clone()),
            ],
            indices: Some(indices.clone()),
        });

        diagnostics.add_measurement(MESH_INDEX_COUNT, indices.len() as f64);

        let vertices = pos_norm_tex_ind
            .positions
            .iter()
            .map(|p| bevy_rapier3d::rapier::math::Point::from_slice(p))
            .collect();
        let indices = indices
            .chunks(3)
            .map(|i| bevy_rapier3d::rapier::na::Point3::<u32>::from_slice(i))
            .collect();

        let body_handle = bodies.insert(RigidBodyBuilder::new_static().build());
        let collider_handle = colliders.insert(
            ColliderBuilder::trimesh(vertices, indices).build(),
            body_handle,
            &mut bodies,
        );

        let entity = commands
            .spawn(PbrComponents {
                mesh,
                material: materials[*i as usize],
                ..Default::default()
            })
            .with_bundle((
                RigidBodyHandleComponent::from(body_handle),
                ColliderHandleComponent::from(collider_handle),
            ))
            .current_entity()
            .unwrap();
        entities.push((entity, mesh, body_handle));
    }
    entities
}

fn generate_meshes(
    mut commands: Commands,
    voxels: ChangedRes<GeneratedVoxelResource>,
    task_pool: Res<ComputeTaskPool>,
    mut diagnostics: ResMut<Diagnostics>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut bodies: ResMut<RigidBodySet>,
    mut colliders: ResMut<ColliderSet>,
    mut joints: ResMut<JointSet>,
    mut voxel_meshes: ResMut<GeneratedMeshesResource>,
    _cam: &GeneratedVoxelsTag,
    cam_transform: &Transform,
) {
    let cam_pos = cam_transform.translation();
    let cam_pos = PointN([cam_pos.x().round() as i32, 0i32, cam_pos.z().round() as i32]);

    let view_distance = voxels.view_distance;
    let chunk_size = voxels.chunk_size;
    let max_height = voxels.max_height;
    let extent = transform_to_extent(cam_pos, view_distance, max_height);
    let extent = extent_modulo_expand(extent, chunk_size);

    let vd2 = view_distance * view_distance;

    let start = std::time::Instant::now();

    let new_meshes = task_pool.scope(|s| {
        let map = &voxels.map;
        let generated_map = &voxel_meshes.generated_map;
        for chunk in map.chunk_keys_for_extent(&extent) {
            s.spawn(async move {
                let p = PointN([chunk.x(), 0, chunk.z()]);
                let d = p - cam_pos;
                if d.dot(&d) > vd2 {
                    // Outside view distance so remove
                    return (None, None, Some(chunk));
                }
                if generated_map.get(&chunk).is_some() {
                    // Already exists so skip
                    return (None, None, None);
                }

                // Generate the new chunk
                (
                    Some(chunk),
                    Some(generate_mesh(map, map.extent_for_chunk_at_key(&chunk))),
                    None,
                )
            })
        }
    });

    let mut mesh_count = 0;
    for (p, mesh, to_remove) in &new_meshes {
        if let Some(p) = to_remove {
            if let Some(entities) = voxel_meshes.generated_map.remove(p) {
                for (entity, mesh, body) in entities {
                    commands.despawn(entity);
                    meshes.remove(&mesh);
                    // NOTE: This removes the body, as well as its colliders and
                    // joints from the simulation so it's the only thing we need to call
                    bodies.remove(body, &mut *colliders, &mut *joints);
                }
            }
        }
        if let Some(p) = p {
            if let Some(mesh) = mesh {
                let mesh_entities = spawn_meshes(
                    &mut commands,
                    &mut diagnostics,
                    &mut meshes,
                    &mut bodies,
                    &mut colliders,
                    &voxels.materials,
                    mesh,
                );
                mesh_count += mesh_entities.len();
                voxel_meshes.generated_map.insert(*p, mesh_entities);
            }
        }
    }

    if mesh_count > 0 {
        let dur = std::time::Instant::now() - start;
        diagnostics.add_measurement(
            MESH_GENERATION_DURATION,
            dur.as_secs_f64() * 1000.0 / mesh_count as f64,
        );
    }
}
