#version 330

out vec4 out_color;
in vec3 UV;
in vec3 Normals;

uniform sampler2D myTexture;

void main() {
    out_color = texture(myTexture, UV.xy);
    //out_color = vec4(0.7, 0.7, 0.2, 1.0);
}