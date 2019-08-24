//! Provides functions for finding the amount of bytes that are processed per second.
//!
//! ```
//! nyx::iter::to_stdout(0..1_000_000_u64).for_each(|_| ())
//! ```

use std::fmt::{self, Display, Formatter};

/// Bytes per second.
///
/// Provides the expected formatting for displaying bytes per second.
///
/// # Examples
/// ```
/// # use nyx::Bps;
/// assert_eq!(Bps(1).to_string(), "1.00 B/s");
/// assert_eq!(Bps(1024).to_string(), "1.00 KiB/s");
/// assert_eq!(Bps(1_048_576).to_string(), "1.00 MiB/s");
/// assert_eq!(Bps(1_073_741_824).to_string(), "1.00 GiB/s");
/// assert_eq!(Bps(1_099_511_627_776).to_string(), "1.00 TiB/s");
/// ```
#[derive(Copy, Clone, Debug, Default, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct Bps(pub u64);

impl Display for Bps {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let n = self.0 as f64;
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

pub mod iter {
    use super::Bps;
    use std::iter::Map;
    use std::mem;
    use std::sync::mpsc::Sender;
    use std::time::Instant;

    /// Returns an iterator that provides the bytes per second by printing it to `stdout`.
    ///
    /// # Examples
    /// ```
    /// nyx::iter::to_stdout(0..1_000_000_u64).for_each(|_| ())
    /// ```
    #[inline]
    pub fn to_stdout<I, T>(
        iter: impl IntoIterator<Item = T, IntoIter = I>,
    ) -> Map<I, impl FnMut(T) -> T>
    where
        I: Iterator<Item = T>,
    {
        to_slot(iter, |bps| println!("{}", bps))
    }

    /// Returns an iterator that provides the bytes per second by calling the provided function.
    ///
    /// # Examples
    /// ```
    /// # use nyx::Bps;
    /// nyx::iter::to_slot(0..1_000_000_u64, |bps| match bps {
    ///     Bps(0) => eprintln!("N/A"),
    ///     bps => println!("{}", bps),
    /// }).for_each(|_| ())
    /// ```
    #[inline]
    pub fn to_slot<I, T>(
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
                f(Bps((bytes as f64 / elapsed.as_secs_f64()) as u64));
                bytes = 0;
            }
            item
        })
    }

    /// Returns an iterator that provides the bytes per second by sending it through the provided `Sender`.
    ///
    /// # Examples
    /// ```
    /// use std::sync::mpsc;
    /// use std::thread;
    ///
    /// let (sender, receiver) = mpsc::channel();
    /// thread::spawn(move || {
    ///     nyx::iter::to_sender(0..1_000_000_u64, sender).for_each(|_| ())
    /// });
    ///
    /// for bps in receiver.iter() {
    ///     println!("{}", bps);
    /// }
    /// ```
    #[inline]
    pub fn to_sender<I, T>(
        iter: impl IntoIterator<Item = T, IntoIter = I>,
        sender: Sender<Bps>,
    ) -> Map<I, impl FnMut(T) -> T>
    where
        I: Iterator<Item = T>,
    {
        to_slot(iter, move |bps| {
            let _ = sender.send(bps);
        })
    }
}
