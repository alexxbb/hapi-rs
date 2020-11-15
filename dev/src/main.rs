mod simple;

fn main() {
    if let Err(e) = simple::run() {
        eprintln!("Error: {}", e)
    }
}
