use bevy::render::{
    mesh::{Mesh, VertexAttribute},
    pipeline::PrimitiveTopology,
};

pub struct Cone {
    pub radius: f32,
    pub segments: usize,
    pub height: f32,
}

impl Default for Cone {
    fn default() -> Self {
        Cone {
            radius: 0.5f32,
            segments: 32,
            height: 1.0f32,
        }
    }
}

impl From<Cone> for Mesh {
    fn from(cone: Cone) -> Self {
        let mut positions = Vec::with_capacity(cone.segments + 2);
        let mut normals = Vec::with_capacity(cone.segments + 2);
        let mut uvs = Vec::with_capacity(cone.segments + 2);
        let mut indices = Vec::with_capacity(cone.segments + 2);

        // bottom
        positions.push([0.0, 0.0, 0.0]);
        normals.push([0.0, -1.0, 0.0]);
        uvs.push([0.5, 0.0]);

        let angle = 360.0f32 / cone.segments as f32;

        // // circular base of cylinder
        // for i in 0..cone.segments {
        //     let x = cone.radius * (angle * i as f32).cos();
        //     let z = cone.radius * (angle * i as f32).sin();
        //     positions.push([x, 0.0, z]);
        //     normals.push([0.0, -1.0, 0.0]);
        //     // FIXME
        //     uvs.push([0.5, 0.0]);
        // }

        // // circular top of cylinder
        // for i in 0..cone.segments {
        //     let x = cone.radius * (angle * i as f32).cos();
        //     let z = cone.radius * (angle * i as f32).sin();
        //     let magnitude = (x * x + z * z).sqrt();
        //     positions.push([x, 0.9, z]);
        //     normals.push([x / magnitude, 0.0, z / magnitude]);
        //     // FIXME
        //     uvs.push([0.5, 0.9]);
        // }

        // circular base of cone
        for i in 0..cone.segments {
            let x = cone.radius * (angle * i as f32).cos();
            let z = cone.radius * (angle * i as f32).sin();
            let magnitude = (x * x + z * z).sqrt();
            positions.push([x, 0.0, z]);
            normals.push([x / magnitude, 0.0, z / magnitude]);
            // FIXME
            uvs.push([0.5, 0.0]);
        }

        // top
        positions.push([0.0, 1.0, 0.0]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([0.5, 1.0]);

        for i in 0..cone.segments {
            indices.append(&mut vec![
                0u32,
                ((i + 1) % cone.segments) as u32,
                ((i + 2) % cone.segments) as u32,
            ]);
            indices.append(&mut vec![
                (cone.segments + 1) as u32,
                ((i + 1) % cone.segments) as u32,
                ((i + 2) % cone.segments) as u32,
            ]);
        }

        Mesh {
            primitive_topology: PrimitiveTopology::TriangleList,
            attributes: vec![
                VertexAttribute::position(positions),
                VertexAttribute::normal(normals),
                VertexAttribute::uv(uvs),
            ],
            indices: Some(indices),
        }
    }
}
