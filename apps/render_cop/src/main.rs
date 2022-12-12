#![allow(dead_code)]
#![allow(unused)]

use std::collections::HashMap;
use std::default::Default;
use std::fmt::{Display, Formatter};
use std::time::Duration;
use hapi_rs::{
    session::{Session, connect_to_pipe, new_in_process, quick_session, SessionOptionsBuilder},
    parameter::*
};
use iced::{Alignment, Element, Length, Sandbox, Settings};
use iced::widget::{column, button, slider, Image, row, Container, Column, text, pick_list};
use iced::widget::image;
use hapi_rs::asset::AssetLibrary;
use hapi_rs::node::{HoudiniNode, NodeFlags, NodeType};
use resolve_path::PathResolveExt;
use anyhow::{anyhow, Context, Result};
use iced::keyboard::KeyCode::Space;
use iced::widget::pane_grid::Axis::Horizontal;

struct App {
    input: f32,
    asset_map: HashMap<Noise, HoudiniNode>,
    image: image::Handle,
    buffer: Vec<u8>,
    noise: Option<Noise>,
    num_cooks: i32,
    render_time: u128,
}

#[derive(Debug, Copy, Clone)]
enum Message {
    InputChanged(f32),
    NoiseSelected(Noise)
}

static TITLE: &str = "Render Houdini COP with Rust/Iced";

fn create_nodes() -> Result<HashMap<Noise, HoudiniNode>> {
    let cwd = std::env::current_dir()?;
    let cwd = cwd.resolve();
    let opt = SessionOptionsBuilder::default().threaded(false).build();
    let session = new_in_process(Some(&opt)).unwrap();
    let lib = session.load_asset_file(cwd.join("cop_render.hda"))?;
    let mut map = HashMap::new();
    map.insert(Noise::Voronoi, lib.create_node("hapi::Cop2/voronoi")?);
    map.insert(Noise::Alligator, lib.create_node("hapi::Cop2/alligator")?);
    Ok(map)
}

fn render_node(node: &HoudiniNode, input: f32) -> (Vec<u8>, u128) {
    let Parameter::Float(parm) = node.parameter("input").expect("Input Parm") else {
        panic!("Parameter input not found");
    };
    parm.set(0, input).unwrap();
    let mut buffer = Vec::new();
    let _start = std::time::Instant::now();
    node.session.render_cop_to_memory(node, &mut buffer,"C", "PNG").expect("COP Render");
    (buffer, _start.elapsed().as_millis())
}

impl Sandbox for App {
    type Message = Message;

    fn new() -> Self {
        fn _setup() -> Result<App> {
            let asset_map = create_nodes()?;
            let (img, ms) = render_node(&asset_map[&Noise::Alligator], 0.0);
            Ok(App {
                input: 0.0,
                asset_map,
                image: image::Handle::from_memory(img),
                buffer: vec![],
                noise: Some(Noise::Alligator),
                num_cooks: 1,
                render_time: ms
            })
        }
        _setup().context("App setup failed").unwrap()
    }

    fn title(&self) -> String {
        TITLE.to_string()
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::InputChanged(val) => {
                self.input = val;
            }
            Message::NoiseSelected(noise) => {self.noise = Some(noise)}
        }
        let node = &self.asset_map[self.noise.as_ref().unwrap()];
        let (image, ms) = render_node(node, self.input);
        self.num_cooks += 1;
        self.render_time = ms;
        self.image = image::Handle::from_memory(image);
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let slider = slider(0.0..=1.0, self.input, Message::Offset).step(0.05).width(Length::Units(300));
        let noise = pick_list(&[Noise::Alligator, Noise::Voronoi][..], self.noise, Message::NoiseSelected);

        let parms = row![
            noise,
            text("Input"),
            slider
        ].spacing(10).align_items(Alignment::Center);
        let stat = row![
            text(format!("Render time: {}ms", self.render_time)),
            text(format!("Node cook count: {}", self.num_cooks))
        ].spacing(20);
        let col = Column::new().spacing(10)
            .push(parms)
            .push(Image::new(self.image.clone()))
            .push(stat);
        Container::new(col).center_x().center_y()
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum Noise {
    Alligator,
    Voronoi
}

impl Display for Noise {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}",
            match self {
                Noise::Alligator => "Alligator",
                Noise::Voronoi => "Voronoi",
            }
        )
    }
}


fn main() -> iced::Result {
    App::run(
        Settings {
            window: iced::window::Settings {
                size: (700, 700),
                resizable: false,
                ..Default::default()
            },
            ..Default::default()
        }
    )
}
