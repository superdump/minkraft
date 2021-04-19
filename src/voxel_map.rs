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

use bevy::{prelude::Res, tasks::ComputeTaskPool};
use building_blocks::{
    prelude::*,
    storage::{ChunkHashMapPyramid3, OctreeChunkIndex, SmallKeyHashMap},
};

use building_blocks::mesh::{IsOpaque, MergeVoxel};
use simdnoise::NoiseBuilder;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Voxel(pub u8);

impl Voxel {
    pub const EMPTY: Self = Self(0);
    pub const FILLED: Self = Self(1);
}

impl IsEmpty for Voxel {
    fn is_empty(&self) -> bool {
        self.0 == 0
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

#[derive(Debug)]
pub struct NoiseConfig {
    frequency: f32,
    scale: f32,
    seed: i32,
    octaves: u8,
}

impl Default for NoiseConfig {
    fn default() -> Self {
        Self {
            frequency: 0.15,
            scale: 20.0,
            seed: 1234,
            octaves: 5,
        }
    }
}

pub fn generate_map(
    pool: &ComputeTaskPool,
    chunks_extent: Extent3i,
    noise_config: Res<NoiseConfig>,
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
        let noise_config = &noise_config;
        for p in chunks_extent.iter_points() {
            s.spawn(async move { generate_chunk(p, noise_config, voxel_map_config) });
        }
    });
    for (chunk_key, chunk) in chunks.into_iter() {
        lod0.write_chunk(chunk_key, chunk);
    }

    let index = OctreeChunkIndex::index_chunk_map(voxel_map_config.superchunk_shape, lod0);

    let world_extent = chunks_extent * voxel_map_config.chunk_shape;
    pyramid.downsample_chunks_with_index(&index, &PointDownsampler, &world_extent);

    VoxelMap { pyramid, index }
}

pub fn generate_chunk(
    key: Point3i,
    noise_config: &Res<NoiseConfig>,
    voxel_map_config: &Res<VoxelMapConfig>,
) -> (Point3i, Array3x1<Voxel>) {
    let chunk_min = key * voxel_map_config.chunk_shape;
    let chunk_extent = Extent3i::from_min_and_shape(chunk_min, voxel_map_config.chunk_shape);
    let mut chunk_noise = Array3x1::fill(chunk_extent, Voxel::EMPTY);

    let noise = noise_array(
        chunk_extent,
        noise_config.frequency,
        noise_config.seed,
        noise_config.octaves,
    );

    // Convert the f32 noise into Voxels.
    let sdf_voxel_noise = TransformMap::new(&noise, |d: f32| {
        if noise_config.scale * d < 0.0 {
            Voxel::FILLED
        } else {
            Voxel::EMPTY
        }
    });
    copy_extent(&chunk_extent, &sdf_voxel_noise, &mut chunk_noise);

    (chunk_min, chunk_noise)
}

fn noise_array(extent: Extent3i, freq: f32, seed: i32, octaves: u8) -> Array3x1<f32> {
    let min = Point3f::from(extent.minimum);
    let (noise, _min_val, _max_val) = NoiseBuilder::fbm_3d_offset(
        min.x(),
        extent.shape.x() as usize,
        min.y(),
        extent.shape.y() as usize,
        min.z(),
        extent.shape.z() as usize,
    )
    .with_seed(seed)
    .with_freq(freq)
    .with_octaves(octaves)
    .generate();

    Array3x1::new_one_channel(extent, noise)
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
        let chunk_log2 = 4;
        let num_lods = 5;
        Self {
            chunk_log2,
            chunk_shape: PointN([1 << chunk_log2; 3]),
            num_lods,
            superchunk_shape: PointN([1 << (chunk_log2 + num_lods as i32 - 1); 3]),
            clip_box_radius: 16,
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
