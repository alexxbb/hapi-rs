#![allow(unused_imports)]
use smol;
use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc::{channel, Receiver, RecvError, Sender};
use std::task::{Context, Poll};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

fn simulate_cooking(prog: Sender<u8>) {
    spawn(move || {
        let cook_for = Duration::from_secs(3);
        let start = Instant::now();
        println!("Server started cooking");
        loop {
            sleep(Duration::from_millis(500));
            prog.send(0);
            println!("Cooking...");
            if (Instant::now() - start) >= cook_for {
                break;
            }
        }
        prog.send(1);
    });
}

fn cook() {
    let (sx, rx) = channel();
    simulate_cooking(sx);
    while let Ok(v) = rx.recv() {
        if v == 1 {
            println!("Cooking done");
            break;
        }
    }
}

struct CookFuture {
    check_cook: Receiver<u8>,
}

impl CookFuture {
    fn new() -> CookFuture {
        let (sx, rx) = channel();
        simulate_cooking(sx);
        CookFuture { check_cook: rx }
    }
    fn ready(&self) -> bool {
        matches!(self.check_cook.recv(), Ok(1))
    }
}

impl Future for CookFuture {
    type Output = CookResult;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.ready() {
            println!("Pending");
            cx.waker().clone().wake();
            Poll::Pending
        } else {
            Poll::Ready(CookResult::Done)
        }
    }
}

enum CookResult {
    Done,
    Cooking,
}

fn cook_async() -> CookFuture {
    CookFuture::new()
}

fn main() {
    // let t = smol::spawn(async {
    //     match cook_async().await {
    //         CookResult::Done => println!("Done"),
    //         CookResult::Cooking => println!("Still cooking"),
    //     }
    // });

    smol::block_on(async {
        println!("Here");
        let c = cook_async();
        println!("Go on");
        c.await;
    });
}
