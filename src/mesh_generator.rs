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
    utilities::bevy_util::{mesh::create_mesh_handle, thread_local_resource::ThreadLocalResource},
    voxel_map::{Voxel, VoxelMap},
};

use bevy_rapier3d::{
    physics::{ColliderHandleComponent, RigidBodyHandleComponent},
    rapier::{
        dynamics::{JointSet, RigidBodyBuilder, RigidBodyHandle, RigidBodySet},
        geometry::{ColliderBuilder, ColliderSet},
    },
};
use building_blocks::{
    mesh::*,
    prelude::*,
    storage::{LodChunkKey3, LodChunkUpdate3, SmallKeyHashMap},
};

use bevy::{asset::prelude::*, ecs, prelude::*, tasks::ComputeTaskPool};
use std::{cell::RefCell, collections::VecDeque};

fn max_mesh_creations_per_frame(pool: &ComputeTaskPool) -> usize {
    40 * pool.thread_num()
}

#[derive(Default)]
pub struct MeshCommandQueue {
    commands: VecDeque<MeshCommand>,
}

impl MeshCommandQueue {
    pub fn enqueue(&mut self, command: MeshCommand) {
        self.commands.push_front(command);
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }
}

// PERF: try to eliminate the use of multiple Vecs
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MeshCommand {
    Create(LodChunkKey3),
    Update(LodChunkUpdate3),
}

#[derive(Default)]
pub struct MeshMaterial(pub Handle<StandardMaterial>);

#[derive(Default)]
pub struct ChunkMeshes {
    // Map from chunk key to mesh entity.
    entities: SmallKeyHashMap<LodChunkKey3, (Entity, Handle<Mesh>, Option<RigidBodyHandle>)>,
}

/// Generates new meshes for all dirty chunks.
pub fn mesh_generator_system(
    mut commands: Commands,
    pool: Res<ComputeTaskPool>,
    voxel_map: Res<VoxelMap>,
    local_mesh_buffers: ecs::system::Local<ThreadLocalMeshBuffers>,
    mesh_material: Res<MeshMaterial>,
    mut mesh_commands: ResMut<MeshCommandQueue>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut chunk_meshes: ResMut<ChunkMeshes>,
    mut bodies: ResMut<RigidBodySet>,
    mut colliders: ResMut<ColliderSet>,
    mut joints: ResMut<JointSet>,
) {
    let new_chunk_meshes = apply_mesh_commands(
        &*voxel_map,
        &*local_mesh_buffers,
        &*pool,
        &mut *mesh_commands,
        &mut *chunk_meshes,
        &mut commands,
        &mut *mesh_assets,
        &mut *bodies,
        &mut *colliders,
        &mut *joints,
    );
    spawn_mesh_entities(
        new_chunk_meshes,
        &*mesh_material,
        &mut commands,
        &mut *mesh_assets,
        &mut *chunk_meshes,
        &mut *bodies,
        &mut *colliders,
        &mut *joints,
    );
}

fn apply_mesh_commands(
    voxel_map: &VoxelMap,
    local_mesh_buffers: &ThreadLocalMeshBuffers,
    pool: &ComputeTaskPool,
    mesh_commands: &mut MeshCommandQueue,
    chunk_meshes: &mut ChunkMeshes,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    bodies: &mut RigidBodySet,
    colliders: &mut ColliderSet,
    joints: &mut JointSet,
) -> Vec<(LodChunkKey3, Option<PosNormMesh>)> {
    let num_chunks_to_mesh = mesh_commands.len().min(max_mesh_creations_per_frame(pool));

    let mut num_creates = 0;
    let mut num_updates = 0;
    pool.scope(|s| {
        let mut num_meshes_created = 0;
        for command in mesh_commands.commands.iter().rev().cloned() {
            match command {
                MeshCommand::Create(key) => {
                    num_creates += 1;
                    num_meshes_created += 1;
                    s.spawn(async move {
                        (
                            key,
                            create_mesh_for_chunk(key, voxel_map, local_mesh_buffers),
                        )
                    });
                }
                MeshCommand::Update(update) => {
                    num_updates += 1;
                    match update {
                        LodChunkUpdate3::Split(split) => {
                            if let Some((entity, mesh, body)) =
                                chunk_meshes.entities.remove(&split.old_chunk)
                            {
                                commands.entity(entity).despawn();
                                meshes.remove(&mesh);
                                if let Some(body) = body {
                                    // NOTE: This removes the body, as well as its colliders and
                                    // joints from the simulation so it's the only thing we need to call
                                    bodies.remove(body, &mut *colliders, &mut *joints);
                                }
                            }
                            for &key in split.new_chunks.iter() {
                                num_meshes_created += 1;
                                s.spawn(async move {
                                    (
                                        key,
                                        create_mesh_for_chunk(key, voxel_map, local_mesh_buffers),
                                    )
                                });
                            }
                        }
                        LodChunkUpdate3::Merge(merge) => {
                            for key in merge.old_chunks.iter() {
                                if let Some((entity, mesh, body)) =
                                    chunk_meshes.entities.remove(&key)
                                {
                                    commands.entity(entity).despawn();
                                    meshes.remove(&mesh);
                                    if let Some(body) = body {
                                        // NOTE: This removes the body, as well as its colliders and
                                        // joints from the simulation so it's the only thing we need to call
                                        bodies.remove(body, &mut *colliders, &mut *joints);
                                    }
                                }
                            }
                            num_meshes_created += 1;
                            s.spawn(async move {
                                (
                                    merge.new_chunk,
                                    create_mesh_for_chunk(
                                        merge.new_chunk,
                                        voxel_map,
                                        local_mesh_buffers,
                                    ),
                                )
                            });
                        }
                    }
                }
            }
            if num_meshes_created >= num_chunks_to_mesh {
                break;
            }
        }

        let new_length = mesh_commands.len() - (num_creates + num_updates);
        mesh_commands.commands.truncate(new_length);
    })
}

fn create_mesh_for_chunk(
    key: LodChunkKey3,
    voxel_map: &VoxelMap,
    local_mesh_buffers: &ThreadLocalMeshBuffers,
) -> Option<PosNormMesh> {
    let chunks = voxel_map.pyramid.level(key.lod);

    let chunk_extent = chunks.indexer.extent_for_chunk_at_key(key.chunk_key);
    let padded_chunk_extent = padded_greedy_quads_chunk_extent(&chunk_extent);

    // Keep a thread-local cache of buffers to avoid expensive reallocations every time we want to mesh a chunk.
    let mesh_tls = local_mesh_buffers.get();
    let mut surface_nets_buffers = mesh_tls
        .get_or_create_with(|| {
            RefCell::new(LocalSurfaceNetsBuffers {
                mesh_buffer: GreedyQuadsBuffer::new(
                    padded_chunk_extent,
                    RIGHT_HANDED_Y_UP_CONFIG.quad_groups(),
                ),
                neighborhood_buffer: Array3x1::fill(padded_chunk_extent, Voxel::EMPTY),
            })
        })
        .borrow_mut();
    let LocalSurfaceNetsBuffers {
        mesh_buffer,
        neighborhood_buffer,
    } = &mut *surface_nets_buffers;

    // While the chunk shape doesn't change, we need to make sure that it's in the right position for each particular chunk.
    neighborhood_buffer.set_minimum(padded_chunk_extent.minimum);

    // Only copy the chunk_extent, leaving the padding empty so that we don't get holes on LOD boundaries.
    copy_extent(&chunk_extent, chunks, neighborhood_buffer);

    let voxel_size = (1 << key.lod) as f32;
    greedy_quads(neighborhood_buffer, &padded_chunk_extent, &mut *mesh_buffer);

    if mesh_buffer.num_quads() == 0 {
        None
    } else {
        let mut mesh = PosNormMesh::default();
        for group in mesh_buffer.quad_groups.iter() {
            for quad in group.quads.iter() {
                group
                    .face
                    .add_quad_to_pos_norm_mesh(&quad, voxel_size, &mut mesh);
            }
        }

        Some(mesh)
    }
}

// ThreadLocal doesn't let you get a mutable reference, so we need to use RefCell. We lock this down to only be used in this
// module as a Local resource, so we know it's safe.
type ThreadLocalMeshBuffers = ThreadLocalResource<RefCell<LocalSurfaceNetsBuffers>>;

pub struct LocalSurfaceNetsBuffers {
    mesh_buffer: GreedyQuadsBuffer,
    neighborhood_buffer: Array3x1<Voxel>,
}

fn spawn_mesh_entities(
    new_chunk_meshes: Vec<(LodChunkKey3, Option<PosNormMesh>)>,
    mesh_material: &MeshMaterial,
    commands: &mut Commands,
    mesh_assets: &mut Assets<Mesh>,
    chunk_meshes: &mut ChunkMeshes,
    mut bodies: &mut RigidBodySet,
    colliders: &mut ColliderSet,
    joints: &mut JointSet,
) {
    for (lod_chunk_key, item) in new_chunk_meshes.into_iter() {
        let old_mesh = if let Some(mesh) = item {
            if mesh.indices.is_empty() {
                None
            } else {
                let indices: Vec<u32> = mesh.indices.iter().map(|i| *i as u32).collect();
                let mesh_handle = create_mesh_handle(&mesh, mesh_assets);
                let entity = commands
                    .spawn_bundle(PbrBundle {
                        mesh: mesh_handle.clone(),
                        material: mesh_material.0.clone(),
                        ..Default::default()
                    })
                    .id();
                let body_handle = if lod_chunk_key.lod == 0 {
                    let collider_vertices = mesh
                        .positions
                        .iter()
                        .cloned()
                        .map(|p| bevy_rapier3d::rapier::math::Point::from_slice(&p))
                        .collect();
                    let collider_indices: Vec<[u32; 3]> =
                        indices.chunks(3).map(|i| [i[0], i[1], i[2]]).collect();

                    let body_handle = bodies.insert(RigidBodyBuilder::new_static().build());
                    let collider_handle = colliders.insert(
                        ColliderBuilder::trimesh(collider_vertices, collider_indices).build(),
                        body_handle,
                        &mut bodies,
                    );

                    commands.entity(entity).insert_bundle((
                        RigidBodyHandleComponent::from(body_handle),
                        ColliderHandleComponent::from(collider_handle),
                    ));

                    Some(body_handle)
                } else {
                    None
                };
                chunk_meshes
                    .entities
                    .insert(lod_chunk_key, (entity, mesh_handle, body_handle))
            }
        } else {
            chunk_meshes.entities.remove(&lod_chunk_key)
        };
        if let Some((entity, mesh, body)) = old_mesh {
            commands.entity(entity).despawn();
            mesh_assets.remove(&mesh);
            if let Some(body) = body {
                // NOTE: This removes the body, as well as its colliders and
                // joints from the simulation so it's the only thing we need to call
                bodies.remove(body, &mut *colliders, &mut *joints);
            }
        }
    }
}
