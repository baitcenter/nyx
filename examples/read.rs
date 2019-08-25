use std::io;

fn main() {
    io::copy(&mut nyx::read::stdout(io::repeat(0)), &mut io::sink()).unwrap();
}
