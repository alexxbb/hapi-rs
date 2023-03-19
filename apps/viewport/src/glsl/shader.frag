#version 330

out vec4 out_color;

//in vec3 UV;
//in vec3 Normal;
//in vec4 Color;
//in vec3 FragPos;

in VS_OUT {
    vec3 UV;
    vec3 Normal;
    vec4 Color;
    vec3 FragPos;
} vs_in;


uniform sampler2D myTexture;
uniform vec3 cameraPos;


void main() {
    vec3 ambient_color = vec3(0.1, 0.05, 0.05);
    vec3 obj_color = vs_in.Color.rgb;
    vec3 light_color = vec3(1.3);
    float specular_strength = 0.3;
    vec3 norm = normalize(vs_in.Normal);
    vec3 camera_dir = normalize(cameraPos - vs_in.FragPos);

    vec3 reflect_dir = reflect(-camera_dir, norm);
    float spec = pow(max(dot(camera_dir, reflect_dir), 0.0), 64.0);
    float diff = max(dot(norm, camera_dir), 0.0);
    vec3 diffuse = diff * light_color;
    vec3 specular = specular_strength * spec * light_color;

    vec4 text_color = texture(myTexture, vs_in.UV.xy);
    vec3 result_color = (text_color.rgb * diffuse + ambient_color + specular) * obj_color;
    out_color.rgb =  result_color;
    out_color.a = 1.0;
}