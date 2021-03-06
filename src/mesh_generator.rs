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
    app_state::AppState,
    fog::FogConfig,
    mesh_fade::{FadeUniform, FADE_IN, FADE_OUT},
    utilities::bevy_util::thread_local_resource::ThreadLocalResource,
    voxel_map::{Voxel, VoxelMap},
};

use bevy_mod_bounding::{aabb::Aabb, obb::Obb};
use bevy_rapier3d::prelude::{ColliderBundle, ColliderShape, RigidBodyBundle, RigidBodyType};
use building_blocks::{
    mesh::*,
    prelude::*,
    storage::{LodChunkKey3, LodChunkUpdate3, SmallKeyHashMap},
};

use bevy::{
    asset::prelude::*,
    ecs,
    prelude::*,
    render::{mesh::Indices, pipeline::PrimitiveTopology},
    tasks::ComputeTaskPool,
};
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

    pub fn clear(&mut self) {
        self.commands.clear();
    }
}

// PERF: try to eliminate the use of multiple Vecs
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MeshCommand {
    Create(LodChunkKey3),
    Update(LodChunkUpdate3),
}

#[derive(Default)]
pub struct ChunkMeshes {
    // Map from chunk key to mesh entity.
    entities: SmallKeyHashMap<LodChunkKey3, (Entity, Handle<Mesh>)>,
    remove_queue: SmallKeyHashMap<LodChunkKey3, (Entity, Handle<Mesh>)>,
}

impl ChunkMeshes {
    pub fn clear_entities(&mut self, commands: &mut Commands, meshes: &mut Assets<Mesh>) {
        self.entities.retain(|_, (entity, mesh)| {
            clear_up_entity(entity, mesh, commands, meshes);
            false
        });
        self.remove_queue.retain(|_, (entity, mesh)| {
            clear_up_entity(entity, mesh, commands, meshes);
            false
        });
    }

    pub fn remove_entity(
        &mut self,
        lod_chunk_key: &LodChunkKey3,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
    ) {
        if let Some((entity, mesh)) = self.entities.remove(lod_chunk_key) {
            clear_up_entity(&entity, &mesh, commands, meshes);
        }
    }
}

fn clear_up_entity(
    entity: &Entity,
    mesh: &Handle<Mesh>,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
) {
    commands.entity(*entity).despawn();
    meshes.remove(mesh);
}

// Utility struct for building the mesh
#[derive(Debug, Clone)]
struct MeshBuf {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub tex_coords: Vec<[f32; 2]>,
    pub layer: Vec<u32>,
    pub indices: Vec<u32>,
    pub extent: Extent3i,
}

impl Default for MeshBuf {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            tex_coords: Vec::new(),
            layer: Vec::new(),
            indices: Vec::new(),
            extent: Extent3i::from_min_and_shape(PointN([0, 0, 0]), PointN([0, 0, 0])),
        }
    }
}

impl MeshBuf {
    fn add_quad(
        &mut self,
        face: &OrientedCubeFace,
        quad: &UnorientedQuad,
        voxel_size: f32,
        u_flip_face: Axis3,
        layer: u32,
    ) {
        let start_index = self.positions.len() as u32;
        self.positions
            .extend_from_slice(&face.quad_mesh_positions(quad, voxel_size));
        self.normals.extend_from_slice(&face.quad_mesh_normals());

        let flip_v = true;
        self.tex_coords
            .extend_from_slice(&face.tex_coords(u_flip_face, flip_v, quad));

        self.layer.extend_from_slice(&[layer; 4]);
        self.indices
            .extend_from_slice(&face.quad_mesh_indices(start_index));
    }
}

pub struct ArrayTextureMaterial(pub Handle<StandardMaterial>);
pub struct ArrayTexturePipelines(pub RenderPipelines);

/// Generates new meshes for all dirty chunks.
pub fn mesh_generator_system(
    mut commands: Commands,
    pool: Res<ComputeTaskPool>,
    voxel_map: Res<VoxelMap>,
    local_mesh_buffers: ecs::system::Local<ThreadLocalMeshBuffers>,
    mut mesh_commands: ResMut<MeshCommandQueue>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut chunk_meshes: ResMut<ChunkMeshes>,
    array_texture_pipelines: Res<ArrayTexturePipelines>,
    array_texture_material: Res<ArrayTextureMaterial>,
    mut state: ResMut<State<AppState>>,
) {
    let first_run = chunk_meshes.entities.is_empty();
    let new_chunk_meshes = apply_mesh_commands(
        &*voxel_map,
        &*local_mesh_buffers,
        &*pool,
        &mut *mesh_commands,
        &mut *chunk_meshes,
        &mut commands,
        first_run,
    );
    spawn_mesh_entities(
        new_chunk_meshes,
        &mut commands,
        &mut *mesh_assets,
        &mut *chunk_meshes,
        &*array_texture_pipelines,
        &*array_texture_material,
    );
    if first_run {
        println!("MESHES GENERATED!\n-> AppState::Running");
        state.set(AppState::Running).unwrap();
    }
}

fn apply_mesh_commands(
    voxel_map: &VoxelMap,
    local_mesh_buffers: &ThreadLocalMeshBuffers,
    pool: &ComputeTaskPool,
    mesh_commands: &mut MeshCommandQueue,
    chunk_meshes: &mut ChunkMeshes,
    commands: &mut Commands,
    first_run: bool,
) -> Vec<(LodChunkKey3, Option<MeshBuf>)> {
    let num_chunks_to_mesh = mesh_commands.len().min(max_mesh_creations_per_frame(pool));

    let mut num_creates = 0;
    let mut num_updates = 0;
    pool.scope(|s| {
        let mut num_meshes_created = 0;
        for command in mesh_commands.commands.iter().rev().cloned() {
            match command {
                MeshCommand::Create(lod_key) => {
                    if !chunk_meshes.entities.contains_key(&lod_key) {
                        num_creates += 1;
                        num_meshes_created += 1;
                        s.spawn(async move {
                            (
                                lod_key,
                                create_mesh_for_chunk(lod_key, voxel_map, local_mesh_buffers),
                            )
                        });
                    }
                }
                MeshCommand::Update(update) => {
                    num_updates += 1;
                    match update {
                        LodChunkUpdate3::Split(split) => {
                            if let Some((entity, mesh)) =
                                chunk_meshes.entities.remove(&split.old_chunk)
                            {
                                chunk_meshes
                                    .remove_queue
                                    .insert(split.old_chunk, (entity, mesh));
                                commands.entity(entity).insert(FADE_OUT);
                            }
                            for &lod_key in split.new_chunks.iter() {
                                if !chunk_meshes.entities.contains_key(&lod_key) {
                                    num_meshes_created += 1;
                                    s.spawn(async move {
                                        (
                                            lod_key,
                                            create_mesh_for_chunk(
                                                lod_key,
                                                voxel_map,
                                                local_mesh_buffers,
                                            ),
                                        )
                                    });
                                }
                            }
                        }
                        LodChunkUpdate3::Merge(merge) => {
                            for lod_key in merge.old_chunks.iter() {
                                if let Some((entity, mesh)) = chunk_meshes.entities.remove(lod_key)
                                {
                                    chunk_meshes.remove_queue.insert(*lod_key, (entity, mesh));
                                    commands.entity(entity).insert(FADE_OUT);
                                }
                            }
                            if !chunk_meshes.entities.contains_key(&merge.new_chunk) {
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
            }
            if !first_run && num_meshes_created >= num_chunks_to_mesh {
                break;
            }
        }

        let new_length = mesh_commands.len() - (num_creates + num_updates);
        mesh_commands.commands.truncate(new_length);
    })
}

pub fn mesh_despawn_system(
    mut commands: Commands,
    mut chunk_meshes: ResMut<ChunkMeshes>,
    mut meshes: ResMut<Assets<Mesh>>,
    query: Query<(&FadeUniform, &LodChunkKey3), With<Handle<Mesh>>>,
) {
    for (fade, lod_chunk_key) in query.iter() {
        if !fade.fade_in && fade.remaining == 0.0 {
            if let Some((entity, mesh)) = chunk_meshes.remove_queue.remove(lod_chunk_key) {
                commands.entity(entity).despawn();
                meshes.remove(&mesh);
            }
        }
    }
}

fn create_mesh_for_chunk(
    key: LodChunkKey3,
    voxel_map: &VoxelMap,
    local_mesh_buffers: &ThreadLocalMeshBuffers,
) -> Option<MeshBuf> {
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
        let mut mesh_buf = MeshBuf::default();
        mesh_buf.extent = chunk_extent * voxel_map.pyramid.chunk_shape();
        for group in mesh_buffer.quad_groups.iter() {
            for quad in group.quads.iter() {
                let mat = neighborhood_buffer.get(quad.minimum);
                mesh_buf.add_quad(
                    &group.face,
                    quad,
                    voxel_size,
                    RIGHT_HANDED_Y_UP_CONFIG.u_flip_face,
                    mat.0 as u32 - 1,
                );
            }
        }

        Some(mesh_buf)
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
    new_chunk_meshes: Vec<(LodChunkKey3, Option<MeshBuf>)>,
    commands: &mut Commands,
    mesh_assets: &mut Assets<Mesh>,
    chunk_meshes: &mut ChunkMeshes,
    array_texture_pipelines: &ArrayTexturePipelines,
    array_texture_material: &ArrayTextureMaterial,
) {
    for (lod_chunk_key, item) in new_chunk_meshes.into_iter() {
        let old_mesh = if let Some(mesh_buf) = item {
            if mesh_buf.indices.is_empty() {
                None
            } else {
                let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);

                let MeshBuf {
                    positions,
                    normals,
                    tex_coords,
                    layer,
                    indices,
                    extent,
                } = mesh_buf;

                render_mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
                render_mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                render_mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, tex_coords);
                render_mesh.set_attribute("Vertex_Layer", layer);
                render_mesh.set_indices(Some(Indices::U32(indices.clone())));

                let mesh_handle = mesh_assets.add(render_mesh);

                let minimum = Vec3::new(
                    extent.minimum.0[0] as f32,
                    extent.minimum.0[1] as f32,
                    extent.minimum.0[2] as f32,
                );
                let maximum = Vec3::new(
                    extent.max().0[0] as f32,
                    extent.max().0[1] as f32,
                    extent.max().0[2] as f32,
                );
                let entity = commands
                    .spawn_bundle(PbrBundle {
                        mesh: mesh_handle.clone(),
                        render_pipelines: array_texture_pipelines.0.clone(),
                        material: array_texture_material.0.clone(),
                        ..Default::default()
                    })
                    .insert_bundle((
                        FADE_IN,
                        lod_chunk_key,
                        Obb::from_aabb_orientation(
                            Aabb::from_extents(minimum, maximum),
                            Quat::IDENTITY,
                        ),
                        FogConfig::default(),
                    ))
                    .id();

                if lod_chunk_key.lod == 0 {
                    let collider_vertices = positions
                        .iter()
                        .cloned()
                        .map(|p| bevy_rapier3d::rapier::math::Point::from_slice(&p))
                        .collect();
                    let collider_indices: Vec<[u32; 3]> =
                        indices.chunks(3).map(|i| [i[0], i[1], i[2]]).collect();

                    commands
                        .entity(entity)
                        .insert_bundle(RigidBodyBundle {
                            body_type: RigidBodyType::Static,
                            ..Default::default()
                        })
                        .insert_bundle(ColliderBundle {
                            shape: ColliderShape::trimesh(collider_vertices, collider_indices),
                            ..Default::default()
                        });
                }
                chunk_meshes
                    .entities
                    .insert(lod_chunk_key, (entity, mesh_handle))
            }
        } else {
            chunk_meshes.entities.remove(&lod_chunk_key)
        };
        if let Some((entity, _mesh)) = old_mesh {
            commands.entity(entity).insert(FADE_OUT);
        }
    }
}
