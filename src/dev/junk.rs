use std::path::Path;

fn join_paths<'a, I>(files: I) -> String
    where I: IntoIterator,
          I::Item: AsRef<Path>
{
    let mut buf = String::new();
    let mut iter = files.into_iter().peekable();
    while let Some(n) = iter.next() {
        buf.push_str(&n.as_ref().to_string_lossy());
        if iter.peek().is_some() {
            buf.push(':');
        }
    }
    buf
}


fn main() {
    let f = ["foo", "bar"];
    let fs = vec!["foo".to_string(), "bar".to_string()];
    let fp = vec![Path::new("foo"), Path::new("bar")];
    println!("Result: {}", join_paths(f.iter()));
    println!("Result: {}", join_paths(fs.iter()));
    println!("Result: {}", join_paths(fp.iter()));
}