#version 450

layout(location = 0) in vec3 uvw;
layout(location = 1) in vec3 local_camera_pos;
layout(location = 2) in vec3 local_pos;
layout(location = 3) in vec3 v_color;

layout(location = 0) out vec4 o_Target;

void main() {
    o_Target = vec4(v_color, 1.0);
}