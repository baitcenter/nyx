fn main() {
    let sum: u64 = nyx::bps_from_iter(0..1_000_000_000_u64).sum();
    println!("{}", sum);
}
