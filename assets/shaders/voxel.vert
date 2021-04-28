// MIT License

// Copyright (c) 2020 Carter Anderson
// Copyright (c) 2020-2021 Robert Swain <robert.swain@gmail.com>

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

// Default bevy PBR shaders with added vertex attribute for texture layer
// and using an array texture for the base colour

#version 450

layout(location = 0) out vec3 v_WorldPosition;
layout(location = 1) out vec3 v_WorldNormal;
layout(location = 2) out vec3 v_Uv;

layout (location = 3) out vec3 v_Uvw;
layout (location = 4) out vec3 v_LocalCameraPos;
layout (location = 5) out vec3 v_LocalPos;

#ifdef STANDARDMATERIAL_NORMAL_MAP
layout(location = 6) out vec4 v_WorldTangent;
#endif

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};
layout(std140, set = 0, binding = 1) uniform CameraPosition {
    vec4 CameraPos;
};

layout(set = 2, binding = 0) uniform Transform {
    mat4 Model;
};

struct VoxelData {
    vec4 position;
    float center_to_edge;
    uint texture_layer;
};
layout(set = 2, binding = 3) buffer VoxelMap_voxels {
    VoxelData[] voxels;
};

void main() {
    uint vx = gl_VertexIndex;
    // 8 vertices per cube so >> 3 to get instance index
    uint instance = vx >> 3;

    vec3 instance_pos = voxels[instance].position.xyz;
    v_LocalCameraPos = CameraPos.xyz - instance_pos;

    uvec3 xyz = uvec3(vx & 0x1, (vx & 0x2) >> 1, (vx & 0x4) >> 2);

    if (v_LocalCameraPos.x > 0) xyz.x = 1 - xyz.x;
    if (v_LocalCameraPos.y > 0) xyz.y = 1 - xyz.y;
    if (v_LocalCameraPos.z > 0) xyz.z = 1 - xyz.z;

    v_Uvw = vec3(xyz);
    vec3 pos = v_Uvw * 2.0 - 1.0;
    v_LocalPos = pos.xyz * voxels[instance].center_to_edge;

    v_WorldPosition = voxels[instance].position.xyz;
    // FIXME
    v_WorldNormal = mat3(Model) * v_LocalPos;
    // FIXME
    v_Uv = vec3(0.5, 0.5, voxels[instance].texture_layer);
#ifdef STANDARDMATERIAL_NORMAL_MAP
    // FIXME
    v_WorldTangent = vec4(mat3(Model) * vec3(1.0), 1.0);
#endif

    gl_Position = ViewProj * vec4(instance_pos + v_LocalPos, 1.0);
}