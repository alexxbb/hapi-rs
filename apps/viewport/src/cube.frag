#version 330

out vec4 out_color;
in vec3 UV;

uniform sampler2D myTexture;

void main() {
    out_color = texture(myTexture, UV.xy);
}