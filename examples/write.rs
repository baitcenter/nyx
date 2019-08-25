use std::io;

fn main() {
    io::copy(&mut io::repeat(0), &mut nyx::write::stdout(io::sink())).unwrap();
}
