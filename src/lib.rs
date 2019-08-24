//! Provides utility for finding throughput in bytes per second.

use std::fmt::{self, Display, Formatter};
use std::iter::Map;
use std::mem;
use std::sync::mpsc::Sender;
use std::time::Instant;

/// Returns an iterator that provides the bytes per second by printing it to `stdout`.
#[inline]
pub fn bps_from_iter<I, T>(
    iter: impl IntoIterator<Item = T, IntoIter = I>,
) -> Map<I, impl FnMut(T) -> T>
where
    I: Iterator<Item = T>,
{
    bps_from_iter_with_slot(iter, move |bps| println!("{}", bps))
}

/// Returns an iterator that provides the bytes per second by calling the provided function.
#[inline]
pub fn bps_from_iter_with_slot<I, T>(
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
        let elapsed = instant.elapsed();
        if elapsed.as_secs() != 0 {
            instant = Instant::now();
            f(Bps((bytes as f32 / elapsed.as_secs_f32()) as u64));
            bytes = 0;
        }
        item
    })
}

/// Returns an iterator that provides the bytes per second by sending it through the provided sender.
#[inline]
pub fn bps_from_iter_with_sender<I, T>(
    iter: impl IntoIterator<Item = T, IntoIter = I>,
    sender: Sender<Bps>,
) -> Map<I, impl FnMut(T) -> T>
where
    I: Iterator<Item = T>,
{
    bps_from_iter_with_slot(iter, move |bps| {
        let _ = sender.send(bps);
    })
}

/// Bytes per second.
///
/// Provides the expected formatting for displaying bytes per second.
///
/// # Examples
/// ```
/// # use nyx::Bps;
/// let bps = Bps(1024);
/// assert_eq!(bps.to_string(), "1.00 KiB/s");
/// ```
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
