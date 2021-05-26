#![allow(unused)]
// mod dev_1;
mod dev_3;
mod dev_4;


// use dev_2::run;
// use dev_3::run;
use dev_4::run;

use env_logger::{fmt::Color, Builder, Env};

fn configure_logger() {
    use std::io::Write;
    let env = Env::default();
    Builder::from_env(env).format(|fmt, record|{
        let mut style = fmt.default_level_style(record.level());
        style.set_color(Color::Yellow).set_bold(false);
        let timestamp = fmt.timestamp();
        writeln!(
            fmt,
            "[{} {}]: {}",
            record.level(),
            record.module_path().unwrap(),
            style.value(record.args())
        )
    }).init()
}


fn main() {
    configure_logger();
    if let Err(e) = unsafe{run()} {
        eprintln!("=====================\n{}======================", e);
        std::process::exit(1)
    }
}
