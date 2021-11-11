use once_cell::sync::OnceCell;
// use flume::Receiver;
use crossbeam_channel::{bounded, Receiver, TryRecvError};
use hapi_rs::session::*;

static POOL: OnceCell<Receiver<Session>> = OnceCell::new();
const NUM_SESSIONS: usize = 20;

fn get_session() -> Session {
    let pool = POOL.get_or_init(|| {
        use env_logger;
        use log;
        env_logger::try_init();
        let (tx, rx) = bounded(NUM_SESSIONS);
        for _ in 0..2 {
            let tx = tx.clone();
            std::thread::spawn(move || loop {
                let session = simple_session(None);
                match session {
                    Ok(session) => {
                        if tx.send(session).is_err() {
                            break;
                        };
                    }
                    Err(e) => {
                        log::warn!("Error: {}", e);
                    }
                }
            });
        }
        rx
    });

    pool.recv().unwrap()
}

fn stress() {
    let (tx, rx) = bounded(NUM_SESSIONS);

    for i in 0..4 {
        let tx_ = tx.clone();
        let h = std::thread::spawn(move || loop {
            let s = simple_session(None).unwrap();
            if tx_.send(s).is_err() {
                break;
            };
        });
    }

    while let Ok(s) = rx.recv() {
        println!("Session !");
    }
}

fn tst() {

    // if let Err(e) = hapi_rs::session::connect_to_pipe("/var/folders/xt/cgm6y73n5g54r3v5f6rrzpxc0000gn/T/.tmpleadcP") {
    //     println!("{}", hapi_rs::session::get_connection_error(false).unwrap());
    // }
}
fn main() {
    tst();
    // for _ in 0..10 {
    //     let s = get_session();
    //     drop(s);
    //     std::thread::sleep(Duration::from_millis(50));
    // }
    // stress();
}
