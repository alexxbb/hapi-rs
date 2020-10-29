use std::rc::Rc;
use std::time::Instant;
struct Foo{s: String}
fn main() {
    let foo = Foo{s: "d".to_string()};
    let rc = Rc::new(foo);

    let s = Instant::now();
    for _ in 0..1000 {
        let clone = rc.clone();
    }
    println!("Elapsed: {}", s.elapsed().as_nanos());
}