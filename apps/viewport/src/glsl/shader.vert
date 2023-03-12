#version 330

layout (location = 0) in vec3 pos;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec3 uv;

uniform mat4 model;
uniform mat4 view;

out VS_OUT {
    vec3 UV;
    vec3 Normal;
    vec3 FragPos;
} vs_out;



void main() {
    gl_Position = view * model * vec4(pos, 1.0);
    vs_out.UV = uv;
    vs_out.Normal = mat3(transpose(inverse(model))) * normal;
    vs_out.FragPos = vec3(model * vec4(pos, 1.0));
}