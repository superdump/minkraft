use crate::types::CameraTag;
use bevy::{
    prelude::*,
    render::{mesh::VertexAttribute, pipeline::PrimitiveTopology},
};
use ilattice3::{prelude::*, ChunkedLatticeMap, ChunkedLatticeMapReader, YLevelsIndexer};
use ilattice3_mesh::{greedy_quads, make_pos_norm_tang_tex_mesh_from_quads, GreedyQuadsVoxel};
use noise::*;
use std::collections::{HashMap, HashSet};

type VoxelMap = ChunkedLatticeMap<Voxel, (), YLevelsIndexer>;

struct GeneratedMeshesResource {
    pub generated_map: HashMap<Point, (Entity, Handle<Mesh>)>,
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
        let chunk_size = 16;
        GeneratedVoxelResource {
            noise: RidgedMulti::new()
                .set_seed(1234)
                .set_frequency(0.01)
                .set_octaves(5),
            chunk_size,
            map: ChunkedLatticeMap::<_, (), YLevelsIndexer>::new(
                [chunk_size, chunk_size, chunk_size].into(),
            ),
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
    res.materials.push(materials.add(Color::GREEN.into()));
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Voxel(u16);

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

impl GreedyQuadsVoxel for Voxel {
    type Material = u16;

    fn material(&self) -> Self::Material {
        self.0
    }
}

fn generate_chunk(res: &mut ResMut<GeneratedVoxelResource>, min: Point, max: Point) {
    let yoffset = 12.0f64;
    let yscale = 10.0f64;
    for z in min.z..max.z {
        for x in min.x..max.x {
            let max_y = (res.noise.get([x as f64, z as f64]) * yscale + yoffset).round() as i32;
            for y in 0..(max_y + 1) {
                let (_p, v) = res
                    .map
                    .get_mut_or_default(&Point::new(x, y, z), (), Voxel(0));
                *v = Voxel(1);
            }
        }
    }
}

fn generate_voxels(
    mut voxels: ResMut<GeneratedVoxelResource>,
    voxel_meshes: Res<GeneratedMeshesResource>,
    _cam: &CameraTag,
    cam_transform: &Transform,
) {
    let cam_pos = cam_transform.translation();
    let cam_pos = Point::new(
        cam_pos.x().round() as i32,
        cam_pos.y().round() as i32,
        cam_pos.z().round() as i32,
    );

    let extent = transform_to_extent(cam_pos, voxels.view_distance);
    let extent = extent_modulo_expand(extent, voxels.chunk_size);
    let min = extent.get_minimum();
    let max = extent.get_world_supremum();

    let chunk_size = voxels.chunk_size;
    let max_height = voxels.max_height;
    let vd2 = voxels.view_distance * voxels.view_distance;
    for z in (min.z..max.z).step_by(voxels.chunk_size as usize) {
        for x in (min.x..max.x).step_by(voxels.chunk_size as usize) {
            let p = Point::new(x, 0, z);
            let d = p - cam_pos;
            if voxel_meshes.generated_map.get(&p).is_some() || d.dot(&d) > vd2 {
                continue;
            }
            generate_chunk(
                &mut voxels,
                Point::new(x, 0, z),
                Point::new(x + chunk_size, max_height, z + chunk_size),
            );
        }
    }
}

fn transform_to_extent(cam_pos: Point, view_distance: i32) -> Extent {
    Extent::from_min_and_world_max(
        [cam_pos.x - view_distance, 0, cam_pos.z - view_distance].into(),
        [cam_pos.x + view_distance, 0, cam_pos.z + view_distance].into(),
    )
}

fn modulo_down(v: i32, modulo: i32) -> i32 {
    (v / modulo) * modulo
}

fn modulo_up(v: i32, modulo: i32) -> i32 {
    ((v / modulo) + 1) * modulo
}

fn extent_modulo_expand(extent: Extent, modulo: i32) -> Extent {
    let min = extent.get_minimum();
    let max = extent.get_world_supremum();
    Extent::from_min_and_world_supremum(
        [
            modulo_down(min.x, modulo),
            min.y,
            modulo_down(min.z, modulo),
        ]
        .into(),
        [
            modulo_up(max.x, modulo) + 1,
            max.y + 1,
            modulo_up(max.z, modulo) + 1,
        ]
        .into(),
    )
}

fn spawn_mesh(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &[Handle<StandardMaterial>],
    voxel_map: &VoxelMap,
    extent: Extent,
) -> (Entity, Handle<Mesh>) {
    let reader = ChunkedLatticeMapReader::new(voxel_map);
    let map = reader
        .map
        .copy_extent_into_new_map(extent, &reader.local_cache);
    let quads = greedy_quads(&map, *map.get_extent());
    let pos_norm_tang_tex_ind = make_pos_norm_tang_tex_mesh_from_quads(&quads);
    let pos_norm_tex_ind = pos_norm_tang_tex_ind.get(&1).unwrap();
    let indices = pos_norm_tex_ind.indices.iter().map(|i| *i as u32).collect();

    let mesh = meshes.add(Mesh {
        primitive_topology: PrimitiveTopology::TriangleList,
        attributes: vec![
            VertexAttribute::position(pos_norm_tex_ind.positions.clone()),
            VertexAttribute::normal(pos_norm_tex_ind.normals.clone()),
            VertexAttribute::uv(pos_norm_tex_ind.tex_coords.clone()),
        ],
        indices: Some(indices),
    });
    let entity = commands
        .spawn(PbrComponents {
            mesh,
            material: materials[1],
            ..Default::default()
        })
        .current_entity()
        .unwrap();
    (entity, mesh)
}

fn generate_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    voxels: ChangedRes<GeneratedVoxelResource>,
    mut voxel_meshes: ResMut<GeneratedMeshesResource>,
    _cam: &CameraTag,
    cam_transform: &Transform,
) {
    let cam_pos = cam_transform.translation();
    let cam_pos = Point::new(
        cam_pos.x().round() as i32,
        cam_pos.y().round() as i32,
        cam_pos.z().round() as i32,
    );

    let view_distance = voxels.view_distance;
    let chunk_size = voxels.chunk_size;
    let extent = transform_to_extent(cam_pos, view_distance);
    let extent = extent_modulo_expand(extent, chunk_size);
    let min = extent.get_minimum();
    let max = extent.get_world_supremum();

    let max_height = voxels.max_height;
    let vd2 = view_distance * view_distance;
    let mut to_remove: HashSet<Point> = voxel_meshes.generated_map.keys().cloned().collect();
    for z in (min.z..max.z).step_by(chunk_size as usize) {
        for x in (min.x..max.x).step_by(chunk_size as usize) {
            let p = Point::new(x, 0, z);
            let d = p - cam_pos;
            if d.dot(&d) > vd2 {
                continue;
            }
            to_remove.remove(&p);
            if voxel_meshes.generated_map.get(&p).is_some() {
                continue;
            }
            let entity_mesh = spawn_mesh(
                &mut commands,
                &mut meshes,
                &voxels.materials,
                &voxels.map,
                Extent::from_minimum_and_local_max(
                    p,
                    Point::new(chunk_size, max_height, chunk_size),
                ),
            );
            voxel_meshes.generated_map.insert(p, entity_mesh);
        }
    }
    for p in &to_remove {
        if let Some((entity, mesh)) = voxel_meshes.generated_map.remove(p) {
            commands.despawn(entity);
            meshes.remove(&mesh);
        }
    }
}
