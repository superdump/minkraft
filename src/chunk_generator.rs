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

use crate::voxel_map::{generate_chunk, NoiseConfig, Voxel, VoxelMap, VoxelMapConfig};

use bevy_prototype_character_controller::controller::CameraTag;
use building_blocks::{core::extent::bounding_extent, prelude::*};

use bevy::{prelude::*, render::camera::Camera, tasks::ComputeTaskPool};
use std::collections::VecDeque;

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
    voxel_map_config: Res<VoxelMapConfig>,
) {
    let num_chunks_to_generate = chunk_commands
        .len()
        .min(max_chunk_creations_per_frame(&pool));

    let mut num_generates = 0;
    let mut num_edits = 0;
    let mut num_removes = 0;
    let mut generated_chunks = pool.scope(|s| {
        let noise_config = &noise_config;
        let voxel_map_config = &voxel_map_config;
        let mut num_chunks_generated = 0;
        for command in chunk_commands.commands.iter().rev().cloned() {
            match command {
                ChunkCommand::Generate(chunk_key) => {
                    num_chunks_generated += 1;
                    s.spawn(
                        async move { generate_chunk(chunk_key, noise_config, voxel_map_config) },
                    );
                }
                _ => {}
            }
            if num_chunks_generated >= num_chunks_to_generate {
                break;
            }
        }
    });
    generated_chunks.reverse();

    let mut generated_chunk_extent: Option<Extent3i> = None;
    {
        let lod0 = voxel_map.pyramid.level_mut(0);
        for command in chunk_commands.commands.iter().rev().cloned() {
            match command {
                ChunkCommand::Generate(chunk_key) => {
                    num_generates += 1;
                    let (voxel_key, chunk) = generated_chunks.pop().unwrap();
                    lod0.write_chunk(voxel_key, chunk);
                    let chunk_extent = Extent3i::from_min_and_shape(chunk_key, Point3i::ONES);
                    if let Some(extent_to_update) = generated_chunk_extent.as_mut() {
                        *extent_to_update = bounding_extent(
                            [
                                extent_to_update.minimum,
                                chunk_extent.minimum,
                                extent_to_update.max(),
                                chunk_extent.max(),
                            ]
                            .iter()
                            .cloned(),
                        );
                    } else {
                        generated_chunk_extent = Some(chunk_extent);
                    }
                }
                ChunkCommand::Edit(chunk_key, chunk) => {
                    num_edits += 1;
                    lod0.write_chunk(chunk_key, chunk);
                }
                ChunkCommand::Remove(chunk_key) => {
                    num_removes += 1;
                }
            }
            if num_generates >= num_chunks_to_generate {
                break;
            }
        }
    }

    if let Some(chunk_extent) = generated_chunk_extent {
        let voxel_extent = chunk_extent * voxel_map_config.chunk_shape;
        pool.scope(|s| {
            let voxel_map = &mut voxel_map;
            s.spawn(async move {
                let mut index = voxel_map.index.clone();
                index.superchunk_octrees.add_extent(&voxel_extent);
                voxel_map.pyramid.downsample_chunks_with_index(
                    &index,
                    &PointDownsampler,
                    &voxel_extent,
                );
                voxel_map.index = index;
            });
        });
    }

    let new_length = chunk_commands.len() - (num_generates + num_edits + num_removes);
    chunk_commands.commands.truncate(new_length);
}

pub fn chunk_detection_system(
    cameras: Query<(&Camera, &GlobalTransform), With<CameraTag>>,
    voxel_map: Res<VoxelMap>,
    voxel_map_config: Res<VoxelMapConfig>,
    mut chunk_commands: ResMut<ChunkCommandQueue>,
) {
    let camera_position = if let Some((_camera, tfm)) = cameras.iter().next() {
        tfm.translation
    } else {
        return;
    };

    let mut camera_center =
        Point3f::from(camera_position).in_voxel() >> voxel_map_config.chunk_log2;
    *camera_center.y_mut() = 0;
    let visible_extent = voxel_map_config.world_chunks_extent + camera_center;

    let lod0 = voxel_map.pyramid.level(0);
    for chunk_key in visible_extent.iter_points() {
        let voxel_key = chunk_key * voxel_map_config.chunk_shape;
        if lod0.get_chunk(voxel_key).is_none() {
            chunk_commands.enqueue(ChunkCommand::Generate(chunk_key));
        }
    }
}
