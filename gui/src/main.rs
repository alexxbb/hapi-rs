use fltk::{
    app::{App, Scheme},
    button,
    enums,
    // menu::*,
    // output::*,
    frame,
    group,
    input,
    valuator,
    window::*,
};

const W: i32 = 600;
const H: i32 = 700;

use hapi_rs::{parameter::*, session::*};

fn yellow_box(x: i32, y: i32, width: i32) -> frame::Frame {
    let mut f = frame::Frame::new(x, y, width, 30, "@+");
    f.set_frame(enums::FrameType::BorderBox);
    f.set_color(enums::Color::Yellow);
    f
}

fn example() {

    let mut root = group::Group::new(10, 10, W - 20, H - 20, "");
    root.set_frame(enums::FrameType::DownFrame);

    for i in 0..5 {
        let mut row = group::Group::new(root.x(), root.y() + (i * 50), W - 20, 30, "");
        row.set_type(group::PackType::Horizontal);
        for j in 0..3 {
            yellow_box(row.x() + (j * 50), row.y(), 50);
        }
        let mut resizable = frame::Frame::new(160, row.y(), W - 170, 50, "Resize");
        resizable.set_color(enums::Color::White);
        resizable.set_frame(frame::FrameType::DownBox);
        row.end();
        row.resizable(&resizable);
    }

    let mut void = frame::Frame::new(root.x(), root.height() - 20, W, 20, "");
    void.hide();
    root.resizable(&void);
    root.end();
}

fn build_ui(parms: Vec<Parameter>) -> Result<group::Group> {
    let mut root = group::Group::new(10, 10, W - 20, H - 20, "");
    root.set_frame(group::FrameType::DownFrame);

    for (i, parm) in parms.iter()
        .take(50)
        .filter(|p|{
            let info = p.info();
            ! matches!(info.parm_type(), ParmType::Folder | ParmType::Folderlist) && ! info.invisible()
        }).enumerate() {
        let info = parm.info();
        let label = info.label()?;
        println!("[{:?}] {}: {}", info.parm_type(), info.name().unwrap(), info.label().unwrap());
        const HEIGHT:i32 = 30;
        let mut row = group::Group::new(root.x(), root.y() + (i as i32 * HEIGHT), root.width(), HEIGHT, "");
        let mut label = frame::Frame::new(row.x(), row.y(), 200, HEIGHT, &label);
        match parm {
            Parameter::Float(_) => {
                yellow_box(label.x() + label.width(), label.y(), root.width() - label.width());
                // if info.size() == 1 {
                //     let mut _slider = valuator::HorNiceSlider::default();
                //     _slider.set_bounds(0.0, 1.0);
                //     _slider.set_range(0.0, 1.0);
                // } else {
                //     for _ in 0..info.size() {
                //         input::Input::default();
                //     }
                // }
            }
            Parameter::Int(_) => {
                yellow_box(label.x() + label.width(), label.y(), root.width() - label.width());
            }
            Parameter::String(_) => {
                yellow_box(label.x() + label.width(), label.y(), root.width() - label.width());
            }
            Parameter::Other(_) => {
                yellow_box(label.x() + label.width(), label.y(), root.width() - label.width());
            }
        }
        row.end();
    }
    let last = root.child(root.children() - 1).unwrap();
    let y = last.y() + last.height();
    let mut void = frame::Frame::new(root.x(), y, root.width(), root.height() - y, "");
    void.hide();
    root.end();
    root.resizable(&void);
    Ok(root)
}

fn run() -> Result<()> {
    let mut session = Session::connect_to_server("/tmp/hapi")?;
    session.initialize(&SessionOptions::default());
    let _lib = session.load_asset_file("/Users/alex/CLionProjects/hapi-rs/otls/hapi_parms.hda")?;
    let node = session.create_node_blocking("Object/hapi_parms", None, None)?;

    let app = App::default().with_scheme(Scheme::Gtk);
    let (screen_width, screen_height) = fltk::app::screen_size();
    let mut wind = Window::new(
        (screen_width / 2.0 - 250.0) as i32,
        (screen_height / 2.0 - 200.0) as i32,
        W,
        H,
        "Hapi Parms",
    );

    build_ui(node.parameters()?);
    wind.make_resizable(true);
    wind.end();
    wind.show();

    app.run().unwrap();
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
    }
}
