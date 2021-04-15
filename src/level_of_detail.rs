/*
 * MIT License
 * 
 * Copyright (c) 2020 bonsairobo
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
    mesh_generator::{MeshCommand, MeshCommandQueue},
    voxel_map::{VoxelMap, CHUNK_LOG2, CLIP_BOX_RADIUS, WORLD_EXTENT},
};

use building_blocks::core::prelude::*;

use bevy::{prelude::*, render::camera::Camera};

#[derive(Default)]
pub struct LodState {
    old_lod0_center: Point3i,
}

impl LodState {
    pub fn new(lod0_center: Point3i) -> Self {
        Self {
            old_lod0_center: lod0_center,
        }
    }
}

/// Adjusts the sample rate of voxels depending on their distance from the camera.
pub fn level_of_detail_system(
    cameras: Query<(&Camera, &Transform)>,
    voxel_map: Res<VoxelMap>,
    mut lod_state: ResMut<LodState>,
    mut mesh_commands: ResMut<MeshCommandQueue>,
) {
    let camera_position = if let Some((_camera, tfm)) = cameras.iter().next() {
        tfm.translation
    } else {
        return;
    };

    let lod0_center = Point3f::from(camera_position).in_voxel() >> CHUNK_LOG2;

    if lod0_center == lod_state.old_lod0_center {
        return;
    }

    voxel_map.index.find_clipmap_chunk_updates(
        &WORLD_EXTENT,
        CLIP_BOX_RADIUS,
        lod_state.old_lod0_center,
        lod0_center,
        |update| mesh_commands.enqueue(MeshCommand::Update(update)),
    );

    lod_state.old_lod0_center = lod0_center;
}
