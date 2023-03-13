#version 330

out vec4 out_color;

in vec3 UV;
in vec3 Normal;
in vec4 Color;
in vec3 FragPos;
in vec3 Dist;


uniform sampler2D myTexture;
uniform vec3 cameraPos;


void main() {
    vec3 ambient_color = vec3(0.05, 0.05, 0.1);
    vec3 obj_color = Color.rgb;
    vec3 light_color = vec3(1.0);
    float specular_strength = 0.7;
    vec3 wire_color = vec3(0.0, 0.0, 0.1);
    vec3 norm = normalize(Normal);
    vec3 camera_dir = normalize(cameraPos - FragPos);

    // Wireframe
    vec3 dist_vec = Dist;
    float d = min(dist_vec[0], min(dist_vec[1], dist_vec[2]));
    float I = exp2(-4.0 * d * d);

    vec3 reflect_dir = reflect(-camera_dir, norm);
    float spec = pow(max(dot(camera_dir, reflect_dir), 0.0), 64.0);
    float diff = max(dot(norm, camera_dir), 0.0);
    vec3 diffuse = diff * light_color;
    vec3 specular = specular_strength * spec * light_color;

    // out_color = texture(myTexture, UV.xy);
    vec3 result_color = (ambient_color + diffuse + specular) * obj_color;
    out_color.rgb = I * wire_color + (1.0 - I) * result_color;
    out_color.a = 1.0;
}