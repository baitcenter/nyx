fn main() {
    nyx::to_stdout(0..1_000_000_000_u64).for_each(|_| ())
}
