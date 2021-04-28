#version 450

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

layout(set = 1, binding = 0) uniform VoxelUBO {
    vec4 VoxelUBO_camera_position;
    vec4 VoxelUBO_center_to_edge;
};

struct VoxelData {
    vec4 position;
    vec4 color;
};
layout(set = 2, binding = 0) buffer VoxelMap_voxels {
    VoxelData[] voxels;
};

layout(location = 0) out vec3 uvw;
layout(location = 1) out vec3 local_camera_pos;
layout(location = 2) out vec3 local_pos;
layout(location = 3) out vec3 v_color;

void main() {
    uint vx = gl_VertexIndex;
    // 8 vertices per cube so >> 3 to get instance index
    uint instance = vx >> 3;

    vec3 instance_pos = voxels[instance].position.xyz;
    local_camera_pos = VoxelUBO_camera_position.xyz - instance_pos;

    uvec3 xyz = uvec3(vx & 0x1, (vx & 0x2) >> 1, (vx & 0x4) >> 2);

    if (local_camera_pos.x > 0) xyz.x = 1 - xyz.x;
    if (local_camera_pos.y > 0) xyz.y = 1 - xyz.y;
    if (local_camera_pos.z > 0) xyz.z = 1 - xyz.z;

    uvw = vec3(xyz);
    vec3 pos = uvw * 2.0 - 1.0;

    local_pos = pos.xyz * VoxelUBO_center_to_edge.xyz;

    v_color = voxels[instance].color.rgb;

    gl_Position = ViewProj * vec4(instance_pos + local_pos, 1.0);
}