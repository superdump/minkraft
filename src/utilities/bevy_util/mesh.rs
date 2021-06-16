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

use building_blocks::mesh::PosNormMesh;

use bevy::{
    prelude::{Assets, Handle},
    render2::{
        mesh::{Indices, Mesh, VertexAttributeValues},
        pipeline::PrimitiveTopology,
    },
};

pub fn create_mesh_handle(mesh: &PosNormMesh, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    assert_eq!(mesh.positions.len(), mesh.normals.len());
    let num_vertices = mesh.positions.len();

    let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);
    render_mesh.set_attribute(
        "Vertex_Position",
        VertexAttributeValues::Float32x3(mesh.positions.clone()),
    );
    render_mesh.set_attribute(
        "Vertex_Normal",
        VertexAttributeValues::Float32x3(mesh.normals.clone()),
    );
    render_mesh.set_attribute(
        "Vertex_Uv",
        VertexAttributeValues::Float32x2(vec![[0.0; 2]; num_vertices]),
    );
    render_mesh.set_indices(Some(Indices::U32(mesh.indices.clone())));

    meshes.add(render_mesh)
}
