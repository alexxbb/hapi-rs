use hapi_rs::enums::{
    AttributeOwner, AttributeTypeInfo, CameraProjectionType, CurveType, ImageDataFormat,
    ImagePacking, PackedPrimInstancingMode, PartType, RSTOrder, StatusVerbosity, StorageType,
    XYZOrder,
};
use hapi_rs::geometry::{
    AttributeInfo, BoxInfo, CameraInfo, CookOptions, CurveInfo, InputCurveInfo, PartInfo,
    SphereInfo, Transform,
};
use hapi_rs::node::TransformEuler;
use hapi_rs::raw::{InputCurveMethod, InputCurveParameterization, TcpPortType};
use hapi_rs::server::ThriftSharedMemoryBufferType;
use hapi_rs::session::{
    CompositorOptions, ImageInfo, SessionInfo, SessionSyncInfo, ThriftServerOptions,
    TimelineOptions, Viewport,
};
use pretty_assertions::assert_eq;

macro_rules! test_struct {
    ($struct:ident, $($field:ident = $value:expr),+ $(,)?) => {
        pastey::paste! {
            #[test]
            fn [<$struct:snake _get_set>]() {
                let mut value = $struct::default();
                $(
                    {
                        let expected = $value;
                        value.[<set_ $field>](expected);
                        assert_eq!(
                            value.$field(),
                            expected,
                            concat!(stringify!($struct), "::", stringify!($field))
                        );
                    }
                )+
            }
        }
    };
}

test_struct!(
    SessionInfo,
    connection_count = 2,
    port_type = TcpPortType::Range,
);

test_struct!(
    ThriftServerOptions,
    auto_close = true,
    timeout_ms = 1500.0,
    verbosity = StatusVerbosity::Statusverbosity1,
    shared_memory_buffer_type = ThriftSharedMemoryBufferType::RingBuffer,
    shared_memory_buffer_size = 1024,
);

test_struct!(
    CompositorOptions,
    max_resolution_x = 1920,
    max_resolution_y = 1080,
);

test_struct!(
    CookOptions,
    split_geo_by_group = true,
    split_geos_by_attribute = true,
    max_vertices_per_primitive = 4,
    refine_curve_to_linear = true,
    curve_refine_lod = 1.5,
    clear_errors_and_warnings = true,
    cook_templated_geos = true,
    split_points_by_vertex_attributes = true,
    handle_box_part_types = true,
    handle_sphere_part_types = true,
    check_part_changes = true,
    cache_mesh_topology = true,
    prefer_output_nodes = true,
    packed_prim_instancing_mode = PackedPrimInstancingMode::Flat,
);

test_struct!(
    AttributeInfo,
    total_array_elements = 8,
    owner = AttributeOwner::Point,
    storage = StorageType::Float,
    tuple_size = 3,
    type_info = AttributeTypeInfo::Vector,
    count = 12,
);

test_struct!(
    PartInfo,
    part_type = PartType::Mesh,
    face_count = 6,
    point_count = 8,
    vertex_count = 24,
    instance_count = 2,
    instanced_part_count = 1,
);

test_struct!(
    TimelineOptions,
    fps = 24.0,
    start_time = 1.0,
    end_time = 120.0,
);

test_struct!(
    CurveInfo,
    curve_type = CurveType::Linear,
    curve_count = 3,
    vertex_count = 12,
    knot_count = 4,
    periodic = true,
    rational = true,
    closed = true,
    has_knots = true,
    order = 4,
);

test_struct!(
    Viewport,
    position = [1.0, 2.0, 3.0],
    rotation = [0.0, 0.0, 0.0, 1.0],
    offset = 5.0,
);

test_struct!(
    Transform,
    position = [1.0, 2.0, 3.0],
    rotation = [0.0, 0.0, 0.0, 1.0],
    scale = [2.0, 2.0, 2.0],
    shear = [0.1, 0.2, 0.3],
    rst_order = RSTOrder::Trs,
);

test_struct!(
    TransformEuler,
    position = [1.0, 2.0, 3.0],
    rotation = [10.0, 20.0, 30.0],
    scale = [2.0, 2.0, 2.0],
    shear = [0.1, 0.2, 0.3],
    roation_order = XYZOrder::Zyx,
    rst_order = RSTOrder::Trs,
);

test_struct!(
    SessionSyncInfo,
    cook_using_houdini_time = true,
    sync_viewport = true,
);

test_struct!(
    BoxInfo,
    center = [1.0, 2.0, 3.0],
    rotation = [10.0, 20.0, 30.0],
    size = [4.0, 5.0, 6.0],
);

test_struct!(SphereInfo, center = [1.0, 2.0, 3.0], radius = 4.5,);

test_struct!(
    CameraInfo,
    focal = 50.0,
    aperture = 36.0,
    pixel_aspect = 1.5,
    focus_distance = 10.0,
    f_stop = 2.8,
    imaging_distance = 100.0,
    res_x = 1920,
    res_y = 1080,
    crop_x = [0.1, 0.9],
    crop_y = [0.2, 0.8],
    win_x = [0.0, 1.0],
    win_y = [0.0, 1.0],
    clip_near = 0.01,
    clip_far = 1000.0,
    shutter_open = -0.25,
    shutter_close = 0.25,
    ortho_zoom = 2.0,
    guide_scale = 1.5,
    projection = CameraProjectionType::Ortho,
);

test_struct!(
    ImageInfo,
    x_res = 1920,
    y_res = 1080,
    gamma = 2.2,
    data_format = ImageDataFormat::Float32,
    interleaved = true,
    packing = ImagePacking::Rgba,
);

test_struct!(
    InputCurveInfo,
    curve_type = CurveType::Nurbs,
    order = 4,
    closed = true,
    reverse = true,
    input_method = InputCurveMethod::Breakpoints,
    breakpoint_parameterization = InputCurveParameterization::Chord,
);
