#![allow(dead_code)]
#![allow(unused)]

use std::default::Default;
use std::fmt::{Display, Formatter};
use std::time::Duration;
use hapi_rs::{
    session::{Session, connect_to_pipe, new_in_process, SessionOptionsBuilder},
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
    offset: f32,
    asset: HoudiniNode,
    image: image::Handle,
    buffer: Vec<u8>,
    noise: Option<Noise>,
    num_cooks: i32,
    render_time: u128,
}

#[derive(Debug, Copy, Clone)]
enum Message {
    Offset(f32),
    NoiseSelected(Noise)
}

static TITLE: &str = "Render Houdini COP with Rust/Iced";

fn setup_houdini() -> Result<HoudiniNode> {
    let cwd = std::env::current_dir()?;
    let cwd = cwd.resolve();
    let opt = SessionOptionsBuilder::default().threaded(false).build();
    let session = new_in_process(Some(&opt)).unwrap();
    let lib = session.load_asset_file(cwd.join("cop_render.hda"))?;
    let asset = lib.try_create_first()?;
    asset.cook_blocking()?;
    Ok(asset)
}

fn render_image(asset: &HoudiniNode, offset: f32, noise: Noise) -> (Vec<u8>, u128) {
    let Parameter::Float(offset_parm) = asset.parameter("offset").expect("Offset Parm") else {
        panic!("Parameter offset not found");
    };
    let Parameter::Int(noise_parm) = asset.parameter("noise").expect("Noise Parm") else {
        panic!("Parameter noise not found");
    };
    offset_parm.set(0, offset).unwrap();
    noise_parm.set(0, match noise {Noise::Aligator => 0, Noise::Voronoi => 1}).unwrap();
    let mut buffer = Vec::new();
    let _start = std::time::Instant::now();
    asset.session.render_cop_to_memory(asset, &mut buffer,"C", "PNG").expect("COP Render");
    (buffer, _start.elapsed().as_millis())
}

impl Sandbox for App {
    type Message = Message;

    fn new() -> Self {
        fn _setup() -> Result<App> {
            let asset = setup_houdini()?;
            let (img, ms) = render_image(&asset, 0.0, Noise::Aligator);
            Ok(App {
                offset: 0.0,
                asset,
                image: image::Handle::from_memory(img),
                buffer: vec![],
                noise: Some(Noise::Aligator),
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
            Message::Offset(val) => {
                self.offset = val;
            }
            Message::NoiseSelected(noise) => {self.noise = Some(noise)}
        }
        let (image, ms) = render_image(&self.asset, self.offset, self.noise.unwrap());
        self.num_cooks += 1;
        self.render_time = ms;
        self.image = image::Handle::from_memory(image);
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let slider = slider(0.0..=1.0, self.offset, Message::Offset).step(0.05).width(Length::Units(300));
        let noise = pick_list(&[Noise::Aligator, Noise::Voronoi][..], self.noise, Message::NoiseSelected);

        let parms = row![
            noise,
            text("Offset"),
            slider
        ].spacing(10).align_items(Alignment::Center);
        let stat = row![
            text(format!("Engine time: {}ms", self.render_time)),
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Noise {
    Aligator,
    Voronoi
}

impl Display for Noise {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}",
            match self {
                Noise::Aligator => "Aligator",
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
