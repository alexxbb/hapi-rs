mod geometry;
mod material;

use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::tasks::{block_on, futures_lite::future};
use bevy::text::FontSmoothing;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use hapi_rs::Result as HapiResult;
use hapi_rs::geometry::Geometry;
use hapi_rs::node::HoudiniNode;
use hapi_rs::parameter::Parameter;
#[allow(unused_imports)]
use hapi_rs::server::{ServerOptions, connect_to_memory_server};
use hapi_rs::session::{SessionOptions, new_in_process_session};

#[derive(Resource)]
struct HoudiniResource {
    asset: HoudiniNode,
    geometry: Geometry,
}

#[derive(Component)]
struct HoudiniMesh {
    animated: bool,
}

#[derive(Default, Debug, Hash, Eq, PartialEq, Clone, States)]
enum HoudiniSetupState {
    #[default]
    Loading,
    Error,
    Ready,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "HAPI Demo".to_string(),
                    ..default()
                }),
                ..default()
            }),
            PanOrbitCameraPlugin,
            FpsOverlayPlugin {
                config: FpsOverlayConfig {
                    text_config: TextFont {
                        // Here we define size of our overlay
                        font_size: 16.0,
                        // If we want, we can use a custom font
                        font: default(),
                        // We could also disable font smoothing,
                        font_smoothing: FontSmoothing::default(),
                    },
                    enabled: true,
                    ..default()
                },
            },
        ))
        .init_state::<HoudiniSetupState>()
        .add_systems(Startup, setup_scene)
        .add_systems(Startup, setup_houdini)
        .add_systems(
            Update,
            get_loading_state.run_if(in_state(HoudiniSetupState::Loading)),
        )
        .add_systems(OnEnter(HoudiniSetupState::Ready), houdini_ready)
        .add_systems(
            Update,
            (input_handler, animate).run_if(in_state(HoudiniSetupState::Ready)),
        )
        .run();
}

fn houdini_ready(
    mut commands: Commands,
    text: Single<Entity, With<LoadingText>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut textures: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    res: Res<HoudiniResource>,
) {
    commands.entity(text.into_inner()).despawn_recursive();

    let HoudiniResource { asset, geometry } = res.into_inner();

    let tex_maps = material::extract_texture_maps(asset, false).expect("texture maps");
    let mesh = geometry::create_bevy_mesh_from_houdini(geometry).expect("Bevy mesh");
    let mesh_handle = meshes.add(mesh);

    commands.spawn((
        HoudiniMesh { animated: true },
        Mesh3d(mesh_handle),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: tex_maps.color.map(|image| textures.add(image)),
            metallic_roughness_texture: tex_maps.specular.map(|image| textures.add(image)),
            normal_map_texture: tex_maps.normal.map(|image| textures.add(image)),
            reflectance: 0.5,
            perceptual_roughness: 1.0,
            ..default()
        })),
    ));
}

#[derive(Debug, Component)]
pub struct LoadingText;

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        LoadingText,
        Text::new("Initializing Houdini...".to_string()),
        TextFont {
            font_size: 50.0,
            ..Default::default()
        },
        Node {
            align_content: AlignContent::Center,
            align_self: AlignSelf::Center,
            ..default()
        },
    ));
    commands.spawn((
        PanOrbitCamera {
            focus: Vec3::new(0.0, 1.0, 0.0),
            orbit_smoothness: 0.6,
            orbit_sensitivity: 1.8,
            button_pan: MouseButton::Middle,
            ..default()
        },
        Transform::from_xyz(3.0, 1.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 1800.0,
            ..default()
        },
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 2_500.,
            ..default()
        },
        Transform::from_xyz(50.0, 50.0, 50.0),
    ));
}

fn animate(
    query: Query<(&Mesh3d, &HoudiniMesh)>,
    session: Res<HoudiniResource>,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let (mesh3d, houdini_mesh) = query.single();
    let HoudiniResource { asset, geometry } = session.as_ref();
    if houdini_mesh.animated {
        let mesh = meshes.get_mut(&mesh3d.0).expect("Houdini mesh");
        if let Parameter::Float(parm) = asset.parameter("time").expect("mod parameter") {
            parm.set(0, time.elapsed_secs()).unwrap();
        }
        update_mesh(mesh, geometry);
    }
}

fn update_mesh(mesh: &mut Mesh, geometry: &Geometry) {
    geometry.node.cook().unwrap();
    let attr = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION).expect("P");
    let geometry::BevyMeshData {
        vertices, normals, ..
    } = geometry::vertex_deform(geometry).unwrap();
    {
        let VertexAttributeValues::Float32x3(p_values) = attr else {
            panic!("P is not Float32x3");
        };
        let _ = std::mem::replace(p_values, vertices.unwrap());
    }
    {
        let attr = mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL).expect("N");
        let VertexAttributeValues::Float32x3(n_values) = attr else {
            panic!("N is not Float32x3");
        };
        let _ = std::mem::replace(n_values, normals.unwrap());
    }
}

fn input_handler(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Single<&mut HoudiniMesh>,
    mut exit: EventWriter<AppExit>,
    res: Res<HoudiniResource>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        query.animated ^= true;
    }
    if keyboard_input.pressed(KeyCode::Escape) {
        exit.send(AppExit::Success);
        res.asset.clone().delete().unwrap()
    }
}

fn init_houdini_resource() -> HapiResult<HoudiniResource> {
    let session_options = SessionOptions::default().threaded(false);
    let session = if cfg!(debug_assertions) {
        let server_options = ServerOptions::shared_memory_with_defaults();
        connect_to_memory_server(server_options, None)?.initialize(session_options)?
    } else {
        new_in_process_session(Some(session_options))?
    };
    let otl = std::path::absolute(std::env::current_dir()?.join("apps/bevy/assets/hda/geo.hda"))?;
    let lib = session.load_asset_file(otl)?;
    let asset = lib.try_create_first()?;
    let geometry = asset.geometry()?.expect("geometry");
    asset.cook()?;
    Ok(HoudiniResource { asset, geometry })
}

#[derive(Resource)]
struct HoudiniStartupTask(Task<HapiResult<HoudiniResource>>);

fn get_loading_state(
    mut task: ResMut<HoudiniStartupTask>,
    text: Single<Entity, With<LoadingText>>,
    mut next_state: ResMut<NextState<HoudiniSetupState>>,
    mut commands: Commands,
) {
    if let Some(resource) = block_on(future::poll_once(&mut task.0)) {
        match resource {
            Ok(resource) => {
                next_state.set(HoudiniSetupState::Ready);
                commands.insert_resource(resource);
            }
            Err(e) => {
                let mut msg = format!("Failed to initialize Houdini:\n\n{:?}", e);
                if cfg!(debug_assertions) {
                    msg.push_str("\nNOTE: Make sure to run in --release mode");
                }
                commands.spawn((
                    Text::new(msg),
                    Node {
                        align_content: AlignContent::Center,
                        align_self: AlignSelf::Center,
                        ..default()
                    },
                ));

                commands.entity(text.into_inner()).despawn_recursive();
                next_state.set(HoudiniSetupState::Error);
            }
        }
        commands.remove_resource::<HoudiniStartupTask>();
    }
}
fn setup_houdini(mut commands: Commands) {
    let task = AsyncComputeTaskPool::get()
        .spawn::<HapiResult<HoudiniResource>>(async move { init_houdini_resource() });
    commands.insert_resource(HoudiniStartupTask(task))
}
