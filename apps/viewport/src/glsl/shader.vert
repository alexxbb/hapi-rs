#version 330

layout (location = 0) in vec3 pos;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec3 color;
layout (location = 3) in vec3 uv;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

uniform bool use_point_color;

out VS_OUT {
    vec3 UV;
    vec3 Normal;
    vec4 Color;
    vec3 FragPos;
} vs_out;



void main() {
    gl_Position = projection * view * model * vec4(pos, 1.0);
    vs_out.UV = uv;
    vs_out.Normal = mat3(transpose(inverse(model))) * normal;
    vs_out.Color = use_point_color ? vec4(color, 1.0) : vec4(1.0);
    vs_out.FragPos = vec3(model * vec4(pos, 1.0));
}