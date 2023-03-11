#version 330

out vec4 out_color;
in vec2 UV;
in vec3 Normals;

uniform sampler2D myTexture;

void main() {
    out_color = texture(myTexture, UV);
}