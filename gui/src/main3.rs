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

fn ui_1() {
    let mut gr = group::Group::new(0, 0, W, 80, "");
    gr.set_frame(frame::FrameType::BorderBox);
    let mut message = frame::Frame::new(0, 0, W, 40, "Message");
    message.set_color(enums::Color::Yellow);
    message.set_frame(frame::FrameType::DownBox);

    let mut rg = group::Group::new(0, 40, W, 40, "");
    let mut resizable = frame::Frame::new(0, 40, W - 40, 40, "Resize");
    resizable.set_color(enums::Color::White);
    resizable.set_frame(frame::FrameType::DownBox);
    let mut fixed = frame::Frame::new(resizable.width(), 40, 40, 40, "Fixed");
    fixed.set_color(enums::Color::Green);
    fixed.set_frame(frame::FrameType::DownBox);
    rg.end();
    rg.resizable(&resizable);

    gr.resizable(&message);
    gr.end();

}

fn ui_2() {
    fn yellow_box(x: i32, y: i32) -> frame::Frame {
        let mut f = frame::Frame::new(x, y, 50, 50, "@+");
        f.set_frame(enums::FrameType::BorderBox);
        f.set_color(enums::Color::Yellow);
        f
    }

    let mut root = group::Group::new(10, 10, W - 20, H - 20, "");
    root.set_frame(enums::FrameType::DownFrame);

    for i in 0..5 {
        let mut row = group::Group::new(root.x(), root.y() + (i * 50), W - 20, 30, "");
        row.set_type(group::PackType::Horizontal);
        for j in 0..3 {
            yellow_box(row.x() + (j * 50), row.y());
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

fn main() {
    let app = App::default().with_scheme(Scheme::Gtk);
    let (screen_width, screen_height) = fltk::app::screen_size();
    let mut wind = Window::new(
        (screen_width / 2.0 - 250.0) as i32,
        (screen_height / 2.0 - 200.0) as i32,
        W,
        H,
        "Hapi Parms",
    );

    // let mut root = group::Group::new(0, 0, W, H, "");
    // root.set_frame(frame::FrameType::BorderBox);
    // ui_1();
    ui_2();
    // let mut end = frame::Frame::new(0, 80, root.width(), root.height(), "");
    // end.set_color(enums::Color::Red);
    // end.set_frame(frame::FrameType::BorderBox);
    // end.hide();
    // root.resizable(&end);
    // root.end();
    wind.make_resizable(true);
    wind.end();
    wind.show();

    app.run().unwrap();
}
