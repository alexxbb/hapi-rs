use fltk::{
    app::{App, Scheme},
    window::*,
    button,
    valuator,
    group,
    frame,
    input,
    enums,
    // menu::*,
    // output::*,
};

const W:i32 = 400;
const H:i32 = 500;

// use hapi_rs::{
//     session::{Session, SessionOptions},
//     parameter::*,
//     errors::*
// };

// fn build_ui(parms: Vec<Parameter>) -> Result<group::Pack> {
//     let mut vpack = group::Pack::default().with_size(500, 400);
//     // let mut vpack = group::Group::default().with_size(500, 400);
//     vpack.set_spacing(10);
//
//     for (i, parm) in parms.iter().take(20).enumerate() {
//         let info = parm.info();
//         let label = info.label()?;
//         if matches!(info.parm_type(), ParmType::Folder) { continue }
//         if info.invisible() { continue }
//         let mut hpack = group::Pack::default().with_size(300, 0);
//         hpack.set_spacing(10);
//         hpack.set_type(group::PackType::Horizontal);
//         frame::Frame::default().with_label(&label);
//
//         match parm {
//             Parameter::Float(_) => {
//                 if info.size() == 1 {
//                     let mut _slider = valuator::HorNiceSlider::default();
//                     _slider.set_bounds(0.0, 1.0);
//                     _slider.set_range(0.0, 1.0);
//                 } else {
//                     for _ in 0..info.size() {
//                         input::Input::default();
//                     }
//                 }
//             }
//             Parameter::Int(_) => {
//                 input::Input::default();
//             }
//             Parameter::String(_) => {
//                 input::Input::default();
//             }
//             Parameter::Other(_) => {
//
//                 button::Button::default();
//             }
//         }
//         hpack.end();
//         if hpack.children() > 0 {
//             hpack.auto_layout();
//         }
//     }
//     vpack.end();
//     vpack.auto_layout();
//
//     Ok(vpack)
// }

fn build_2() {
    fn yellow_box() -> frame::Frame {
        let mut f = frame::Frame::default().with_label("@+");
        f.set_pos(20, 20);
        f.set_size(50, 50);
        f.set_frame(enums::FrameType::BorderBox);
        f.set_color(enums::Color::Yellow);
        f
    }
    let mut root = group::Group::new(10, 10, W - 20, H - 20, "");
    root.set_frame(enums::FrameType::DownFrame);
    yellow_box();

    // let mut vpack = group::Pack::new(10, 10, W - 10, H - 10, "");
    // vpack.set_type(group::PackType::Vertical);
    // vpack.set_spacing(10);
    //
    // for i in 0..5 {
    //     let mut row = group::Pack::new(0, 0, 480, 30, "");
    //     row.set_spacing(10);
    //     row.set_type(group::PackType::Horizontal);
    //     for j in 0..3 {
    //         yellow_box();
    //     }
    //     row.end();
    //     row.resizable(&row);
    //     if row.children() > 0 {
    //         row.auto_layout();
    //     }
    // }
    //
    // vpack.end();
    root.end();

}

fn main() {
    // let mut session = Session::connect_to_server("/tmp/hapi")?;
    // session.initialize(&SessionOptions::default());
    // let _lib = session.load_asset_file("/Users/alex/CLionProjects/hapi-rs/otls/hapi_parms.hda")?;
    // let node = session.create_node_blocking("Object/hapi_parms", None, None)?;
    //
    let app = App::default().with_scheme(Scheme::Gtk);
    let (screen_width, screen_height) = fltk::app::screen_size();
    let mut wind = Window::new(
        (screen_width / 2.0 - 250.0) as i32,
        (screen_height / 2.0 - 200.0) as i32,
        W,
        H,
        "Hapi Parms",
    );

    // let scroll = group::Scroll::new(0, 0, W, H, "");
    build_2();
    // let mut hor = group::Group::new(0, 0, W, H, "");
    // hor.set_type(group::PackType::Horizontal);
    // hor.resizable(&hor);
    // hor.end();
    // scroll.end();
    wind.make_resizable(true);
    wind.end();
    wind.show();

    app.run().unwrap();
}
