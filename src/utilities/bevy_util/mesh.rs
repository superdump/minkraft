use building_blocks::mesh::PosNormMesh;

use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        pipeline::PrimitiveTopology,
    },
};

pub fn create_mesh_handle(mesh: &PosNormMesh, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    assert_eq!(mesh.positions.len(), mesh.normals.len());
    let num_vertices = mesh.positions.len();

    let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);
    render_mesh.set_attribute(
        "Vertex_Position",
        VertexAttributeValues::Float3(mesh.positions.clone()),
    );
    render_mesh.set_attribute(
        "Vertex_Normal",
        VertexAttributeValues::Float3(mesh.normals.clone()),
    );
    render_mesh.set_attribute(
        "Vertex_Uv",
        VertexAttributeValues::Float2(vec![[0.0; 2]; num_vertices]),
    );
    render_mesh.set_indices(Some(Indices::U32(mesh.indices.clone())));

    meshes.add(render_mesh)
}
