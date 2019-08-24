//! Provides utility for finding throughput in bytes per second.
//!
//! # Examples
//! ```
//! nyx::throughput(0..10_000_000, |bps| { dbg!(bps); });
//! ```

use std::fmt::{self, Display, Formatter};
use std::iter::Map;
use std::mem;
use std::time::Instant;

/// Returns a new iterator that yields a callback of the bytes processed per second from the
/// given iterator.
#[inline]
pub fn throughput<I, T>(
    iter: impl IntoIterator<Item = T, IntoIter = I>,
    mut f: impl FnMut(Bps),
) -> Map<I, impl FnMut(T) -> T>
where
    I: Iterator<Item = T>,
{
    let mut bytes = 0;
    let mut instant = Instant::now();
    iter.into_iter().map(move |item| {
        bytes += mem::size_of_val(&item) as u64;
        let seconds = instant.elapsed().as_secs();
        if seconds != 0 {
            instant = Instant::now();
            f(Bps(bytes / seconds));
            bytes = 0;
        }
        item
    })
}

/// Bytes per second.
#[derive(Copy, Clone, Debug, Default, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct Bps(pub u64);

impl Display for Bps {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let n = self.0 as f32;
        match self.0 {
            0..=1023 => write!(f, "{:.2} B/s", n),
            1024..=1_048_575 => write!(f, "{:.2} KiB/s", n / 1024.0),
            1_048_576..=1_073_741_823 => write!(f, "{:.2} MiB/s", n / 1_048_576.0),
            1_073_741_824..=1_099_511_627_775 => write!(f, "{:.2} GiB/s", n / 1_073_741_824.0),
            1_099_511_627_776..=18_446_744_073_709_551_615 => {
                write!(f, "{:.2} TiB/s", n / 1_099_511_627_776.0)
            }
        }
    }
}
