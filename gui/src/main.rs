use relm::{connect, Relm, Update, Widget};
use relm_derive::*;
use gtk::prelude::*;
use gtk::{Window, Inhibit, WindowType};

use hapi_rs::{
    session::{Session, SessionOptions},
    parameter::{Parameter, ParmType},
    node::HoudiniNode
};

struct Model {
    node: HoudiniNode
}

struct Win {
    model: Model,
    window: Window,
}

#[derive(Msg)]
enum Msg {
    Quit
}

impl Update for Win {
    type Model = Model;
    type ModelParam = HoudiniNode;
    type Msg = Msg;

    fn model(relm: &Relm<Self>, param: Self::ModelParam) -> Self::Model {
        Model{node: param}
    }

    fn update(&mut self, event: Self::Msg) {
        match event {
            Msg::Quit => gtk::main_quit()
        }
    }
}

impl Widget for Win {
    type Root = Window;

    fn root(&self) -> Self::Root {
        self.window.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let window = Window::new(WindowType::Toplevel);
        // Connect the signal `delete_event` to send the `Quit` message.
        connect!(relm, window, connect_delete_event(_, _), return (Some(Msg::Quit), Inhibit(false)));
        let cont = gtk::Box::new(gtk::Orientation::Vertical, 5);
        let p = model.node.parameters();
        for n in &model.node.parameters().unwrap() {
            let info = n.info();
            match info.parm_type() {
                ParmType::Float => if info.size() == 1 {
                    let lb = gtk::Label::new(Some(&info.name().unwrap()));
                    let fl = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 0.1);
                    fl.set_property_expand(true);
                    let b = gtk::Box::new(gtk::Orientation::Horizontal, 3);
                    b.add(&lb);
                    b.add(&fl);
                    cont.add(&b);
                }
                _ => {
                    let lb = gtk::Label::new(Some(&n.name().unwrap()));
                    cont.add(&lb);
                }

            }
        }
        window.add(&cont);
        window.show_all();
        window.present();
        Win {
            model,
            window,
        }
    }
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut session = Session::connect_to_server("/tmp/hapi")?;
    session.initialize(&SessionOptions::default());
    let _lib = session.load_asset_file("/Users/alex/CLionProjects/hapi-rs/otls/hapi_parms.hda")?;
    let node = session.create_node_blocking("Object/hapi_parms", None, None)?;
    Win::run(node.clone()).expect("Win::run failed");
    Ok(())
}
