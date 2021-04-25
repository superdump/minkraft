/*
 * MIT License
 *
 * Copyright (c) 2020 bonsairobo
 * Copyright (c) 2021 Robert Swain <robert.swain@gmail.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 *
 */

use bevy::{prelude::*, render::camera::Camera, tasks::ComputeTaskPool};
use bevy_prototype_character_controller::controller::CameraTag;
use bevy_rapier3d::rapier::{
    dynamics::{JointSet, RigidBodySet},
    geometry::ColliderSet,
};
use building_blocks::{
    prelude::*,
    storage::{ChunkHashMapPyramid3, OctreeChunkIndex, SmallKeyHashMap},
};

use building_blocks::mesh::{IsOpaque, MergeVoxel};
use simdnoise::NoiseBuilder;

use crate::{
    app_state::AppState,
    chunk_generator::{chunk_detection_system, chunk_generator_system, ChunkCommandQueue},
    level_of_detail::{level_of_detail_system, LodState},
    mesh_fade::mesh_fade_update_system,
    mesh_generator::{
        mesh_despawn_system, mesh_generator_system, ChunkMeshes, MeshCommand, MeshCommandQueue,
    },
};

pub struct VoxelMapPlugin;

impl Plugin for VoxelMapPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(NoiseConfig::default())
            .insert_resource(VoxelMapConfig::default())
            .insert_resource(ChunkCommandQueue::default())
            .insert_resource(MeshCommandQueue::default())
            .add_system_set(
                SystemSet::on_update(AppState::Running)
                    .with_system(
                        voxel_map_config_update_system
                            .system()
                            .label("voxel_map_config_update"),
                    )
                    .with_system(
                        voxel_map_config_changed_system
                            .system()
                            .label("voxel_map_config_changed")
                            .after("voxel_map_config_update"),
                    )
                    .with_system(
                        chunk_detection_system
                            .system()
                            .label("chunk_detection")
                            .after("voxel_map_config_changed"),
                    )
                    .with_system(
                        chunk_generator_system
                            .system()
                            .label("chunk_generator")
                            .after("chunk_detection"),
                    )
                    .with_system(
                        level_of_detail_system
                            .system()
                            .label("level_of_detail")
                            .after("chunk_generator"),
                    )
                    .with_system(
                        mesh_generator_system
                            .system()
                            .label("mesh_generator")
                            .after("level_of_detail"),
                    )
                    .with_system(
                        mesh_fade_update_system
                            .system()
                            .label("mesh_fade_update")
                            .before("mesh_generator"),
                    )
                    .with_system(
                        mesh_despawn_system
                            .system()
                            .label("mesh_despawn")
                            .after("mesh_fade_update"),
                    ),
            );
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Voxel(pub u8);

impl Voxel {
    pub const EMPTY: Self = Self(0);
    pub const WATER: Self = Self(1);
    pub const SAND: Self = Self(2);
    pub const GRASS: Self = Self(3);
    pub const DIRT: Self = Self(4);
    pub const STONE: Self = Self(5);
    pub const SNOW: Self = Self(6);
}

impl IsEmpty for Voxel {
    fn is_empty(&self) -> bool {
        *self == Voxel::EMPTY
    }
}

impl IsOpaque for Voxel {
    fn is_opaque(&self) -> bool {
        true
    }
}

impl MergeVoxel for Voxel {
    type VoxelValue = u8;

    fn voxel_merge_value(&self) -> Self::VoxelValue {
        self.0
    }
}

pub struct VoxelMap {
    pub pyramid: ChunkHashMapPyramid3<Voxel>,
    pub index: OctreeChunkIndex,
}

impl VoxelMap {
    pub fn new(
        pool: &Res<ComputeTaskPool>,
        voxel_map_config: &Res<VoxelMapConfig>,
        noise_config: &Res<NoiseConfig>,
        mut mesh_commands: ResMut<MeshCommandQueue>,
        lod0_center: Point3i,
    ) -> VoxelMap {
        println!(
            "Generating map with {} LODs of {:?} chunks...",
            voxel_map_config.num_lods, voxel_map_config.chunk_shape
        );
        // Generate a voxel map from noise.
        let map = generate_map(
            pool,
            voxel_map_config.world_chunks_extent,
            noise_config,
            voxel_map_config,
        );
        println!("...DONE!!!");

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
}

#[derive(Debug)]
pub struct NoiseConfig {
    frequency: f32,
    seed: i32,
    octaves: u8,
    y_offset: f32,
    y_scale: f32,
}

impl Default for NoiseConfig {
    fn default() -> Self {
        Self {
            frequency: 1.0 / 256.0,
            seed: 1234,
            octaves: 5,
            y_offset: 128.0,
            y_scale: 1024.0,
        }
    }
}

pub struct VoxelMapConfig {
    pub chunk_log2: i32,
    pub chunk_shape: Point3i,
    pub num_lods: u8,
    pub superchunk_shape: Point3i,
    pub clip_box_radius: i32,
    pub world_chunks_extent: Extent3i,
    pub world_voxel_extent: Extent3i,
}

const CHUNKS_MINIMUM_XZ: i32 = -50;
const CHUNKS_MINIMUM_Y: i32 = 0;
const CHUNKS_SHAPE: i32 = 100;
const CHUNKS_THICKNESS: i32 = 1;

impl Default for VoxelMapConfig {
    fn default() -> Self {
        let chunk_log2 = 5;
        let num_lods = 6;
        let clip_box_radius = 8;
        VoxelMapConfig::new(chunk_log2, num_lods, clip_box_radius)
    }
}

impl VoxelMapConfig {
    pub fn new(chunk_log2: i32, num_lods: u8, clip_box_radius: i32) -> VoxelMapConfig {
        VoxelMapConfig {
            chunk_log2,
            chunk_shape: PointN([1 << chunk_log2; 3]),
            num_lods,
            superchunk_shape: PointN([1 << (chunk_log2 + num_lods as i32 - 1); 3]),
            clip_box_radius,
            world_chunks_extent: Extent3i {
                minimum: PointN([CHUNKS_MINIMUM_XZ, CHUNKS_MINIMUM_Y, CHUNKS_MINIMUM_XZ]),
                shape: PointN([CHUNKS_SHAPE, CHUNKS_THICKNESS, CHUNKS_SHAPE]),
            },
            world_voxel_extent: Extent3i {
                minimum: PointN([
                    CHUNKS_MINIMUM_XZ << chunk_log2,
                    CHUNKS_MINIMUM_Y << chunk_log2,
                    CHUNKS_MINIMUM_XZ << chunk_log2,
                ]),
                shape: PointN([
                    CHUNKS_SHAPE << chunk_log2,
                    CHUNKS_THICKNESS << chunk_log2,
                    CHUNKS_SHAPE << chunk_log2,
                ]),
            },
        }
    }
}

const MAX_CLIP_BOX_RADIUS: i32 = 32;
const MAX_CHUNK_LOG2: i32 = 6;
// NOTE: Maximum number of LODs supported by building-blocks ChunkPyramidMap is 6
// due to using an OctreeSet for a 'superchunk' and OctreeSet LocationCodes are limited
// to 6 levels.
const MAX_LODS: u8 = 6;

pub fn voxel_map_config_update_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut voxel_map_config: ResMut<VoxelMapConfig>,
) {
    if keyboard_input.just_pressed(KeyCode::R) {
        voxel_map_config.clip_box_radius <<= 1;
        if voxel_map_config.clip_box_radius > MAX_CLIP_BOX_RADIUS {
            voxel_map_config.clip_box_radius = 1;
        }
        println!("Clip box radius: {}", voxel_map_config.clip_box_radius);
    }
    if keyboard_input.just_pressed(KeyCode::C) {
        voxel_map_config.chunk_log2 += 1;
        if voxel_map_config.chunk_log2 > MAX_CHUNK_LOG2 {
            voxel_map_config.chunk_log2 = 1;
        }
        println!("Chunk log2: {}", voxel_map_config.chunk_log2);
        *voxel_map_config = VoxelMapConfig::new(
            voxel_map_config.chunk_log2,
            voxel_map_config.num_lods,
            voxel_map_config.clip_box_radius,
        );
    }
    if keyboard_input.just_pressed(KeyCode::L) {
        voxel_map_config.num_lods += 1;
        if voxel_map_config.num_lods > MAX_LODS {
            voxel_map_config.num_lods = 1;
        }
        println!("Number of LoDs: {}", voxel_map_config.num_lods);
        *voxel_map_config = VoxelMapConfig::new(
            voxel_map_config.chunk_log2,
            voxel_map_config.num_lods,
            voxel_map_config.clip_box_radius,
        );
    }
}

pub fn voxel_map_config_changed_system(
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
    if voxel_map_config.is_changed() && !voxel_map_config.is_added() {
        chunk_meshes.clear_entities(
            &mut commands,
            &mut meshes,
            &mut bodies,
            &mut colliders,
            &mut joints,
        );
        mesh_commands.clear();

        let camera_position = if let Some((_camera, tfm)) = cameras.iter().next() {
            tfm.translation
        } else {
            return;
        };

        let lod0_center = Point3f::from(camera_position).in_voxel() >> voxel_map_config.chunk_log2;

        *voxel_map = VoxelMap::new(
            &pool,
            &voxel_map_config,
            &noise_config,
            mesh_commands,
            lod0_center,
        );
        lod_state.old_lod0_center = lod0_center;
    }
}

pub fn generate_map(
    pool: &Res<ComputeTaskPool>,
    chunks_extent: Extent3i,
    noise_config: &Res<NoiseConfig>,
    voxel_map_config: &Res<VoxelMapConfig>,
) -> VoxelMap {
    let builder = ChunkMapBuilder3x1::new(voxel_map_config.chunk_shape, Voxel::EMPTY);
    let mut pyramid = ChunkHashMapPyramid3::new(
        builder,
        || SmallKeyHashMap::new(),
        voxel_map_config.num_lods,
    );
    let lod0 = pyramid.level_mut(0);

    let chunks = pool.scope(|s| {
        for x in chunks_extent.minimum.x()..chunks_extent.least_upper_bound().x() {
            for z in chunks_extent.minimum.z()..chunks_extent.least_upper_bound().z() {
                let p = PointN([x, 0, z]);
                s.spawn(async move { generate_chunk_stack(p, noise_config, voxel_map_config) });
            }
        }
    });
    for (chunk_key, chunk) in chunks.into_iter().flatten() {
        lod0.write_chunk(chunk_key, chunk);
    }

    let index = OctreeChunkIndex::index_chunk_map(voxel_map_config.superchunk_shape, lod0);

    let world_extent = lod0.bounding_extent();
    pyramid.downsample_chunks_with_index(&index, &PointDownsampler, &world_extent);

    VoxelMap { pyramid, index }
}

fn index(p: Point3i, shape: Point3i) -> usize {
    (p.z() * shape.z() + p.x()) as usize
}

fn scale_noise(v: f32, config: &NoiseConfig) -> f32 {
    (v - 4.5) * config.y_scale + config.y_offset
}

pub fn generate_chunk_stack(
    key: Point3i,
    noise_config: &Res<NoiseConfig>,
    voxel_map_config: &Res<VoxelMapConfig>,
) -> Vec<(Point3i, Array3x1<Voxel>)> {
    let chunk_min = key * voxel_map_config.chunk_shape;
    let chunk_voxel_extent = Extent3i::from_min_and_shape(chunk_min, voxel_map_config.chunk_shape);

    let (noise, min_y, max_y) = NoiseBuilder::ridge_2d_offset(
        chunk_voxel_extent.minimum.x() as f32,
        chunk_voxel_extent.shape.x() as usize,
        chunk_voxel_extent.minimum.z() as f32,
        chunk_voxel_extent.shape.z() as usize,
    )
    .with_seed(noise_config.seed)
    .with_freq(noise_config.frequency)
    .with_octaves(noise_config.octaves)
    .generate();

    let mut chunks = Vec::new();

    let min_y_chunk = (scale_noise(min_y, &noise_config) as i32) >> voxel_map_config.chunk_log2;
    let max_y_chunk = (scale_noise(max_y, &noise_config) as i32) >> voxel_map_config.chunk_log2;
    for y_min_chunk in min_y_chunk..=max_y_chunk {
        let y_min = y_min_chunk << voxel_map_config.chunk_log2;
        let y_chunk_min = PointN([chunk_min.x(), y_min, chunk_min.z()]);
        let y_chunk_voxel_extent =
            Extent3i::from_min_and_shape(y_chunk_min, voxel_map_config.chunk_shape);
        let mut chunk_noise = Array3x1::fill(y_chunk_voxel_extent, Voxel::EMPTY);
        chunk_noise.for_each_mut(&y_chunk_voxel_extent, |p: Point3i, v: &mut Voxel| {
            let local_p = p - chunk_min;
            let noise_index = index(local_p, voxel_map_config.chunk_shape);
            if (p.y() as f32) < scale_noise(noise[noise_index], &noise_config) {
                *v = height_to_material(p.y(), &noise_config);
            }
        });
        chunks.push((y_chunk_min, chunk_noise));
    }

    chunks
}

// FIXME: Make this more generic - take a config for the thresholds
fn height_to_material(y: i32, config: &NoiseConfig) -> Voxel {
    match y as f32 {
        y if y < scale_noise(4.52, config) => Voxel::WATER,
        y if y < scale_noise(4.54, config) => Voxel::SAND,
        y if y < scale_noise(4.55, config) => Voxel::DIRT,
        y if y < scale_noise(4.7, config) => Voxel::GRASS,
        y if y < scale_noise(4.8, config) => Voxel::STONE,
        _ => Voxel::SNOW,
    }
}
