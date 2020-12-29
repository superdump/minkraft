use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, Mesh},
        pipeline::PrimitiveTopology,
    },
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

        let angle = 2.0f32 * std::f32::consts::PI / cone.segments as f32;

        // circular base of cone
        let frac_h_2 = Vec3::new(0.0f32, 0.5f32 * cone.height, 0.0f32);
        for i in 0..cone.segments {
            let (z, x) = (angle * i as f32).sin_cos();
            let (z, x) = (cone.radius * z, cone.radius * x);
            let position = Vec3::new(x, 0.0f32, z);
            positions.push(*position.as_ref());
            let normal = (position - frac_h_2).normalize();
            normals.push(*normal.as_ref());
            // FIXME
            uvs.push([0.5, 0.0]);
        }

        // top
        positions.push([0.0, cone.height, 0.0]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([0.5, 1.0]);

        for i in 0..cone.segments {
            // bottom circle
            indices.append(&mut vec![
                0u32,
                (1 + (i % cone.segments)) as u32,
                (1 + ((i + 1) % cone.segments)) as u32,
            ]);
            // cone
            indices.append(&mut vec![
                (cone.segments + 1) as u32,
                (1 + ((i + 1) % cone.segments)) as u32,
                (1 + (i % cone.segments)) as u32,
            ]);
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(Indices::U32(indices)));
        mesh
    }
}

pub struct Cylinder {
    pub radius: f32,
    pub segments: usize,
    pub height: f32,
}

impl Default for Cylinder {
    fn default() -> Self {
        Cylinder {
            radius: 0.5f32,
            segments: 32,
            height: 1.0f32,
        }
    }
}

impl From<Cylinder> for Mesh {
    fn from(cylinder: Cylinder) -> Self {
        let mut positions = Vec::with_capacity(cylinder.segments + 2);
        let mut normals = Vec::with_capacity(cylinder.segments + 2);
        let mut uvs = Vec::with_capacity(cylinder.segments + 2);
        let mut indices = Vec::with_capacity(cylinder.segments + 2);

        // bottom
        positions.push([0.0, 0.0, 0.0]);
        normals.push([0.0, -1.0, 0.0]);
        uvs.push([0.5, 0.0]);

        // top
        positions.push([0.0, cylinder.height, 0.0]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([0.5, 1.0]);

        let angle = 2.0f32 * std::f32::consts::PI / cylinder.segments as f32;

        // circular base of cylinder
        for i in 0..cylinder.segments {
            let (z, x) = (angle * i as f32).sin_cos();
            let (z, x) = (cylinder.radius * z, cylinder.radius * x);
            let magnitude = (x * x + z * z).sqrt();
            positions.push([x, 0.0, z]);
            normals.push([x / magnitude, 0.0, z / magnitude]);
            // FIXME
            uvs.push([0.5, 0.0]);
        }

        // circular top of cylinder
        for i in 0..cylinder.segments {
            let (z, x) = (angle * i as f32).sin_cos();
            let (z, x) = (cylinder.radius * z, cylinder.radius * x);
            let magnitude = (x * x + z * z).sqrt();
            positions.push([x, cylinder.height, z]);
            normals.push([x / magnitude, 0.0, z / magnitude]);
            // FIXME
            uvs.push([0.5, cylinder.height]);
        }

        for i in 0..cylinder.segments {
            let bottom_offset = 2;
            let top_offset = 2 + cylinder.segments;

            // bottom circle
            indices.append(&mut vec![
                0u32,
                (bottom_offset + (i % cylinder.segments)) as u32,
                (bottom_offset + ((i + 1) % cylinder.segments)) as u32,
            ]);

            // cylinder
            indices.append(&mut vec![
                (bottom_offset + ((i + 1) % cylinder.segments)) as u32,
                (bottom_offset + (i % cylinder.segments)) as u32,
                (top_offset + (i % cylinder.segments)) as u32,
            ]);
            indices.append(&mut vec![
                (top_offset + (i % cylinder.segments)) as u32,
                (top_offset + ((i + 1) % cylinder.segments)) as u32,
                (bottom_offset + ((i + 1) % cylinder.segments)) as u32,
            ]);

            // top circle
            indices.append(&mut vec![
                1u32,
                (top_offset + ((i + 1) % cylinder.segments)) as u32,
                (top_offset + (i % cylinder.segments)) as u32,
            ]);
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(Indices::U32(indices)));
        mesh
    }
}
