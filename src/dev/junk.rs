struct Foo{}
fn main() {
    let foo = Foo{};
    let p1 = &foo as *const Foo;
    let p2 = p1;
    let p3 = p2;
    dbg!([p1, p2, p3]);
}