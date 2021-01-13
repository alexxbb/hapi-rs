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

fn create_parm(parm: &Parameter) -> Result<gtk::Widget, Box::<dyn std::error::Error>> {
    let info = parm.info();
    let cont = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    let n = format!("{}-{}", &info.label()?, &info.name()?);
    let label = gtk::Label::new(Some(&n));
    cont.add(&label);
    match parm {
        Parameter::Float(p) => {
            if info.size() == 1 {
                let fl = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 0.1);
                fl.set_property_expand(true);
                cont.add(&fl);
            } else {
                let adj = None::<&gtk::Adjustment>;
                for i in 0..info.size() {
                    let adj = gtk::Adjustment::new(0.1, 0.0, 10.0, 0.001, 0.1, 0.0);
                    let p = gtk::SpinButton::new(Some(&adj), 0.01, 3);
                    p.set_property_expand(true);
                    cont.add(&p)
                }
            }
        }
        Parameter::Int(p) => {
            if matches!(info.parm_type(), ParmType::Button) {
                let p = gtk::Button::with_label(&info.name()?);
                cont.add(&p);
            } else {
                let p = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 10.0, 1.0);
                p.set_property_expand(true);
                cont.add(&p);
            }
        }
        Parameter::String(p) => {
            let p = gtk::Entry::new();
            p.set_property_expand(true);
            cont.add(&p);
        }
        Parameter::Other(p) => {
            let p = gtk::Label::new(Some("Other Parm"));
            p.set_property_expand(true);
            cont.add(&p);
        }
    }
    Ok(cont.upcast::<gtk::Widget>())
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
        for n in &model.node.parameters().unwrap() {
            if n.info().invisible() {
                continue
            }
            let parm = create_parm(n).expect("Could not create parm");
            cont.add(&parm);
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
