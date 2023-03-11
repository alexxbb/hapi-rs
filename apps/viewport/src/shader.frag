#version 330

in vec3 UV;
in vec3 Normal;
in vec3 FragPos;

out vec4 out_color;

uniform sampler2D myTexture;
uniform vec3 cameraPos;


void main() {
    vec3 ambient_color = vec3(0.05, 0.05, 0.1);
    vec3 obj_color = vec3(0.8, 0.5, 0.7);
    vec3 light_color = vec3(1.0);
    float specularStrength = 0.7;

    vec3 norm = normalize(Normal);
    vec3 camera_dir = normalize(cameraPos - FragPos);

    vec3 reflect_dir = reflect(-camera_dir, norm);

    float spec = pow(max(dot(camera_dir, reflect_dir), 0.0), 64.0);

    float diff = max(dot(norm, camera_dir), 0.0);

    vec3 diffuse = diff * light_color;
    vec3 specular = specularStrength * spec * light_color;

    // out_color = texture(myTexture, UV.xy);
    vec3 result_color = (ambient_color + diffuse + specular) * obj_color;
    out_color = vec4(result_color, 1.0);
}