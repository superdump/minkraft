use bevy::{
    prelude::*,
    render::{mesh::Indices, pipeline::PrimitiveTopology},
};
use bevy_rapier3d::{
    physics::{ColliderHandleComponent, RigidBodyHandleComponent},
    rapier::{
        dynamics::{JointSet, RigidBodyBuilder, RigidBodyHandle, RigidBodySet},
        geometry::{ColliderBuilder, ColliderSet},
    },
};
use building_blocks::{
    core::prelude::*,
    mesh::{greedy_quads, GreedyQuadsBuffer, MaterialVoxel},
    storage::{prelude::*, IsEmpty},
};
use noise::{MultiFractal, NoiseFn, RidgedMulti, Seedable};
use std::collections::{HashMap, HashSet};

const SEA_LEVEL: f64 = 64.0;
const TERRAIN_Y_SCALE: f64 = 0.2;

type VoxelMap = ChunkHashMap3<Voxel>;
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
    pub noise: RidgedMulti,
    pub chunk_size: i32,
    pub map: VoxelMap,
    pub max_height: i32,
    pub view_distance: i32,
    pub materials: Vec<Handle<StandardMaterial>>,
}

impl Default for GeneratedVoxelResource {
    fn default() -> Self {
        let chunk_size = 32;
        GeneratedVoxelResource {
            noise: RidgedMulti::new()
                .set_seed(1234)
                .set_frequency(0.008)
                .set_octaves(5),
            chunk_size,
            map: ChunkMapBuilder3 {
                chunk_shape: PointN([chunk_size; 3]),
                ambient_value: Voxel(0),
                default_chunk_metadata: (),
            }
            .build_with_hash_map_storage(),
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

fn generate_chunk(res: &mut ResMut<GeneratedVoxelResource>, min: Point3i, max: Point3i) {
    let yoffset = SEA_LEVEL;
    let yscale = TERRAIN_Y_SCALE * yoffset;
    for z in min.z()..max.z() {
        for x in min.x()..max.x() {
            let max_y = (res.noise.get([x as f64, z as f64]) * yscale + yoffset).round() as i32;
            for y in 0..(max_y + 1) {
                *res.map.get_mut(&PointN([x, y, z])) = Voxel(height_to_material(y));
            }
        }
    }
}

fn generate_voxels(
    mut voxels: ResMut<GeneratedVoxelResource>,
    voxel_meshes: Res<GeneratedMeshesResource>,
    query: Query<&Transform, With<GeneratedVoxelsTag>>,
) {
    for cam_transform in query.iter() {
        let cam_pos = cam_transform.translation;
        let cam_pos = PointN([cam_pos.x.round() as i32, 0i32, cam_pos.z.round() as i32]);

        let extent = transform_to_extent(cam_pos, voxels.view_distance);
        let extent = extent_modulo_expand(extent, voxels.chunk_size);
        let min = extent.minimum;
        let max = extent.least_upper_bound();

        let chunk_size = voxels.chunk_size;
        let max_height = voxels.max_height;
        let vd2 = voxels.view_distance * voxels.view_distance;
        for z in (min.z()..max.z()).step_by(voxels.chunk_size as usize) {
            for x in (min.x()..max.x()).step_by(voxels.chunk_size as usize) {
                let p = PointN([x, 0, z]);
                let d = p - cam_pos;
                if voxel_meshes.generated_map.get(&p).is_some() || d.dot(&d) > vd2 {
                    continue;
                }
                generate_chunk(
                    &mut voxels,
                    PointN([x, 0, z]),
                    PointN([x + chunk_size, max_height, z + chunk_size]),
                );
            }
        }
    }
}

fn transform_to_extent(cam_pos: Point3i, view_distance: i32) -> Extent3i {
    Extent3i::from_min_and_lub(
        PointN([cam_pos.x() - view_distance, 0, cam_pos.z() - view_distance]),
        PointN([cam_pos.x() + view_distance, 0, cam_pos.z() + view_distance]),
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

fn spawn_mesh(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mut bodies: &mut ResMut<RigidBodySet>,
    colliders: &mut ResMut<ColliderSet>,
    materials: &[Handle<StandardMaterial>],
    voxel_map: &VoxelMap,
    extent: Extent3i,
) -> Vec<(Entity, Handle<Mesh>, RigidBodyHandle)> {
    let padded_chunk_extent = extent.padded(1);
    let mut padded_chunk = Array3::fill(padded_chunk_extent, Voxel(0));
    copy_extent(&padded_chunk_extent, voxel_map, &mut padded_chunk);
    let mut buffer = GreedyQuadsBuffer::new(padded_chunk_extent);
    greedy_quads(&padded_chunk, &padded_chunk_extent, &mut buffer);

    let mut pos_norm_tex_inds = HashMap::new();
    for group in buffer.quad_groups.iter() {
        for (quad, material) in group.quads.iter() {
            group.face.add_quad_to_pos_norm_tex_mesh(
                quad,
                pos_norm_tex_inds.entry(*material).or_default(),
            );
        }
    }

    let mut entities = Vec::with_capacity(meshes.len());
    for (material, pos_norm_tex_ind) in pos_norm_tex_inds {
        let indices: Vec<u32> = pos_norm_tex_ind.indices.iter().map(|i| *i as u32).collect();

        let collider_vertices = pos_norm_tex_ind
            .positions
            .iter()
            .map(|p| bevy_rapier3d::rapier::math::Point::from_slice(p))
            .collect();
        let collider_indices = indices
            .chunks(3)
            .map(|i| bevy_rapier3d::rapier::na::Point3::<u32>::from_slice(i))
            .collect();

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, pos_norm_tex_ind.positions.clone());
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, pos_norm_tex_ind.normals.clone());
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, pos_norm_tex_ind.tex_coords.clone());
        mesh.set_indices(Some(Indices::U32(indices)));
        let mesh_handle = meshes.add(mesh);

        let body_handle = bodies.insert(RigidBodyBuilder::new_static().build());
        let collider_handle = colliders.insert(
            ColliderBuilder::trimesh(collider_vertices, collider_indices).build(),
            body_handle,
            &mut bodies,
        );

        let entity = commands
            .spawn(PbrBundle {
                mesh: mesh_handle.clone(),
                material: materials[material as usize].clone(),
                ..Default::default()
            })
            .with_bundle((
                RigidBodyHandleComponent::from(body_handle),
                ColliderHandleComponent::from(collider_handle),
            ))
            .current_entity()
            .unwrap();
        entities.push((entity, mesh_handle, body_handle));
    }
    entities
}

fn generate_meshes(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut bodies: ResMut<RigidBodySet>,
    mut colliders: ResMut<ColliderSet>,
    mut joints: ResMut<JointSet>,
    voxels: ChangedRes<GeneratedVoxelResource>,
    mut voxel_meshes: ResMut<GeneratedMeshesResource>,
    query: Query<&Transform, With<GeneratedVoxelsTag>>,
) {
    for cam_transform in query.iter() {
        let cam_pos = cam_transform.translation;
        let cam_pos = PointN([cam_pos.x.round() as i32, 0i32, cam_pos.z.round() as i32]);

        let view_distance = voxels.view_distance;
        let chunk_size = voxels.chunk_size;
        let extent = transform_to_extent(cam_pos, view_distance);
        let extent = extent_modulo_expand(extent, chunk_size);
        let min = extent.minimum;
        let max = extent.least_upper_bound();

        let max_height = voxels.max_height;
        let vd2 = view_distance * view_distance;
        let mut to_remove: HashSet<Point3i> = voxel_meshes.generated_map.keys().cloned().collect();
        for z in (min.z()..max.z()).step_by(chunk_size as usize) {
            for x in (min.x()..max.x()).step_by(chunk_size as usize) {
                let p = PointN([x, 0, z]);
                let d = p - cam_pos;
                if d.dot(&d) > vd2 {
                    continue;
                }
                to_remove.remove(&p);
                if voxel_meshes.generated_map.get(&p).is_some() {
                    continue;
                }
                let entity_mesh = spawn_mesh(
                    commands,
                    &mut meshes,
                    &mut bodies,
                    &mut colliders,
                    &voxels.materials,
                    &voxels.map,
                    Extent3i::from_min_and_shape(p, PointN([chunk_size, max_height, chunk_size])),
                );
                voxel_meshes.generated_map.insert(p, entity_mesh);
            }
        }
        for p in &to_remove {
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
    }
}
