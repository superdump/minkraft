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

use crate::{
    voxel_map::{
        generate_chunk, NoiseConfig, Voxel, VoxelMap, CHUNK_LOG2, CHUNK_SHAPE, SUPERCHUNK_SHAPE,
        WORLD_CHUNKS_EXTENT,
    },
};

use bevy_prototype_character_controller::controller::CameraTag;
use building_blocks::{
    core::extent::bounding_extent,
    prelude::*,
};

use bevy::{prelude::*, render::camera::Camera, tasks::ComputeTaskPool};
use std::{collections::VecDeque};

fn max_chunk_creations_per_frame(pool: &ComputeTaskPool) -> usize {
    40 * pool.thread_num()
}

#[derive(Default)]
pub struct ChunkCommandQueue {
    commands: VecDeque<ChunkCommand>,
}

impl ChunkCommandQueue {
    pub fn enqueue(&mut self, command: ChunkCommand) {
        self.commands.push_front(command);
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChunkCommand {
    Generate(Point3i),
    Edit(Point3i, Array3x1<Voxel>),
    Remove(Point3i),
}

/// Generates / removes chunks
pub fn chunk_generator_system(
    pool: Res<ComputeTaskPool>,
    mut voxel_map: ResMut<VoxelMap>,
    mut chunk_commands: ResMut<ChunkCommandQueue>,
    noise_config: Res<NoiseConfig>,
) {
    let num_chunks_to_generate = chunk_commands
        .len()
        .min(max_chunk_creations_per_frame(&pool));

    let mut num_generates = 0;
    let mut num_edits = 0;
    let mut num_removes = 0;
    let mut generated_chunks = pool.scope(|s| {
        let noise_config = &noise_config;
        let mut num_chunks_generated = 0;
        for command in chunk_commands.commands.iter().rev().cloned() {
            match command {
                ChunkCommand::Generate(key) => {
                    num_chunks_generated += 1;
                    s.spawn(async move { generate_chunk(key, noise_config) });
                }
                // ChunkCommand::Edit(key, chunk) => {
                //     num_edits += 1;
                // }
                // ChunkCommand::Remove(key) => {
                //     num_removes += 1;
                // }
                _ => {}
            }
            if num_chunks_generated >= num_chunks_to_generate {
                break;
            }
        }
    });
    generated_chunks.reverse();

    let mut extent_to_update: Option<Extent3i> = None;
    {
        let lod0 = voxel_map.pyramid.level_mut(0);
        for command in chunk_commands.commands.iter().rev().cloned() {
            match command {
                ChunkCommand::Generate(key) => {
                    num_generates += 1;
                    let (chunk_key, chunk) = generated_chunks.pop().unwrap();
                    // println!("k {:?} ck {:?}", key, chunk_key);
                    debug_assert!(key * CHUNK_SHAPE == chunk_key);
                    lod0.write_chunk(chunk_key, chunk);
                    let extent = Extent3i::from_min_and_shape(key, Point3i::ONES);
                    if let Some(extent_to_update) = extent_to_update.as_mut() {
                        *extent_to_update = bounding_extent(
                            [
                                extent_to_update.minimum,
                                extent.minimum,
                                extent_to_update.max(),
                                extent.max(),
                            ]
                            .iter()
                            .cloned(),
                        );
                    } else {
                        extent_to_update = Some(extent);
                    }
                }
                ChunkCommand::Edit(key, chunk) => {
                    num_edits += 1;
                    lod0.write_chunk(key, chunk);
                }
                ChunkCommand::Remove(key) => {
                    num_removes += 1;
                }
                _ => {}
            }
            if num_generates >= num_chunks_to_generate {
                break;
            }
        }
    }

    if let Some(extent) = extent_to_update {
        let voxel_map = &mut voxel_map;
        pool.scope(|s| {
            let lod0 = voxel_map.pyramid.level(0);
            let index = OctreeChunkIndex::index_chunk_map(SUPERCHUNK_SHAPE, lod0);
            let world_extent = extent * CHUNK_SHAPE;
            s.spawn(async move {
                voxel_map.pyramid.downsample_chunks_with_index(
                    &index,
                    &PointDownsampler,
                    &world_extent,
                );
            });
        });
    }

    let new_length = chunk_commands.len() - (num_generates + num_edits + num_removes);
    chunk_commands.commands.truncate(new_length);
}

pub fn chunk_detection_system(
    cameras: Query<(&Camera, &GlobalTransform), With<CameraTag>>,
    voxel_map: Res<VoxelMap>,
    mut chunk_commands: ResMut<ChunkCommandQueue>,
) {
    let camera_position = if let Some((_camera, tfm)) = cameras.iter().next() {
        tfm.translation
    } else {
        return;
    };

    let camera_center = Point3f::from(camera_position).in_voxel() >> CHUNK_LOG2;
    let camera_center = PointN([camera_center.x(), 0, camera_center.z()]);
    let visible_extent = WORLD_CHUNKS_EXTENT + camera_center;

    let lod0 = voxel_map.pyramid.level(0);
    for p in visible_extent.iter_points() {
        let key = p * CHUNK_SHAPE;
        if lod0.get_chunk(key).is_none() {
            chunk_commands.enqueue(ChunkCommand::Generate(key));
        }
    }
}
