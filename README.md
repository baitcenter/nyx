![nyx](nyx.svg)

[![Travis](https://travis-ci.org/evenorog/nyx.svg?branch=master)](https://travis-ci.org/evenorog/nyx)
[![Crates.io](https://img.shields.io/crates/v/nyx.svg)](https://crates.io/crates/nyx)
[![Docs](https://docs.rs/nyx/badge.svg)](https://docs.rs/nyx)

Provides functions for finding the amount of bytes that are processed per second
by iterators, readers, and writers.

## Examples

Add this to `Cargo.toml`:

```toml
[dependencies]
nyx = "0.1"
```

And this to `main.rs`:

```rust
use std::io;

fn main() {
    io::copy(&mut nyx::read::stdout(io::repeat(0)), &mut io::sink()).unwrap();
}
```

This will write the amount of bytes copied per second to `stdout` in one second intervals.

```
28.06 GiB/s
29.34 GiB/s
30.06 GiB/s
29.33 GiB/s
```

### License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
