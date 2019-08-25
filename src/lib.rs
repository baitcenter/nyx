//! Provides functions for finding the amount of bytes that are processed per second
//! by iterators, readers, and writers.
//!
//! # Examples
//!
//! ```no_run
//! use std::io;
//!
//! fn main() {
//!     io::copy(&mut nyx::read::stdout(io::repeat(0)), &mut io::sink()).unwrap();
//! }
//! ```
//!
//! Outputs something like this forever:
//!
//! ```text
//! 28.06 GiB/s
//! 29.34 GiB/s
//! 30.06 GiB/s
//! 29.33 GiB/s
//! ```

#![doc(html_root_url = "https://docs.rs/nyx/latest")]
#![deny(
    bad_style,
    bare_trait_objects,
    missing_docs,
    unused_import_braces,
    unused_qualifications,
    unsafe_code,
    unstable_features
)]

use std::fmt::{self, Display, Formatter};
use std::time::Instant;

/// Bytes per second with expected formatting.
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

#[inline]
fn step(new: u64, sum: &mut u64, instant: &mut Instant, mut f: impl FnMut(Bps)) {
    *sum += new;
    let elapsed = instant.elapsed();
    if elapsed.as_secs() != 0 {
        *instant = Instant::now();
        f(Bps((*sum as f64 / elapsed.as_secs_f64()) as u64));
        *sum = 0;
    }
}

/// Methods for iterators.
pub mod iter {
    use crate::Bps;
    use std::iter::Map;
    use std::mem;
    use std::sync::mpsc::Sender;
    use std::time::Instant;

    /// Returns an iterator that provides the bytes per second by printing it to `stdout`.
    #[inline]
    pub fn stdout<I, T>(i: impl IntoIterator<Item = T, IntoIter = I>) -> Map<I, impl FnMut(T) -> T>
    where
        I: Iterator<Item = T>,
    {
        slot(i, |bps| println!("{}", bps))
    }

    /// Returns an iterator that provides the bytes per second by sending it through the provided `Sender`.
    #[inline]
    pub fn sender<I, T>(
        i: impl IntoIterator<Item = T, IntoIter = I>,
        sender: Sender<Bps>,
    ) -> Map<I, impl FnMut(T) -> T>
    where
        I: Iterator<Item = T>,
    {
        slot(i, move |bps| {
            let _ = sender.send(bps);
        })
    }

    /// Returns an iterator that provides the bytes per second by calling the provided function.
    #[inline]
    pub fn slot<I, T>(
        i: impl IntoIterator<Item = T, IntoIter = I>,
        mut f: impl FnMut(Bps),
    ) -> Map<I, impl FnMut(T) -> T>
    where
        I: Iterator<Item = T>,
    {
        let mut bytes = 0;
        let mut instant = Instant::now();
        i.into_iter().map(move |item| {
            crate::step(
                mem::size_of_val(&item) as u64,
                &mut bytes,
                &mut instant,
                &mut f,
            );
            item
        })
    }
}

/// Methods for readers.
pub mod read {
    use crate::Bps;
    use std::io::{self, Read};
    use std::sync::mpsc::Sender;
    use std::time::Instant;

    #[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
    struct Reader<R, F> {
        r: R,
        f: F,
        bytes: u64,
        instant: Instant,
    }

    impl<R: Read, F: FnMut(Bps)> Read for Reader<R, F> {
        #[inline]
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let bytes = self.r.read(buf)?;
            crate::step(
                bytes as u64,
                &mut self.bytes,
                &mut self.instant,
                &mut self.f,
            );
            Ok(bytes)
        }
    }

    /// Returns a reader that provides the bytes per second by printing it to `stdout`.
    #[inline]
    pub fn stdout(r: impl Read) -> impl Read {
        slot(r, |bps| println!("{}", bps))
    }

    /// Returns a reader that provides the bytes per second by sending it through the provided `Sender`.
    #[inline]
    pub fn sender(r: impl Read, sender: Sender<Bps>) -> impl Read {
        slot(r, move |bps| {
            let _ = sender.send(bps);
        })
    }

    /// Returns a reader that provides the bytes per second by calling the provided function.
    #[inline]
    pub fn slot(r: impl Read, f: impl FnMut(Bps)) -> impl Read {
        Reader {
            r,
            f,
            bytes: 0,
            instant: Instant::now(),
        }
    }
}

/// Methods for writers.
pub mod write {
    use crate::Bps;
    use std::io::{self, Write};
    use std::sync::mpsc::Sender;
    use std::time::Instant;

    #[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
    struct Writer<W, F> {
        w: W,
        f: F,
        bytes: u64,
        instant: Instant,
    }

    impl<W: Write, F: FnMut(Bps)> Write for Writer<W, F> {
        #[inline]
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let bytes = self.w.write(buf)?;
            crate::step(
                bytes as u64,
                &mut self.bytes,
                &mut self.instant,
                &mut self.f,
            );
            Ok(bytes)
        }

        #[inline]
        fn flush(&mut self) -> io::Result<()> {
            self.w.flush()
        }
    }

    /// Returns a writer that provides the bytes per second by printing it to `stdout`.
    #[inline]
    pub fn stdout(w: impl Write) -> impl Write {
        slot(w, |bps| println!("{}", bps))
    }

    /// Returns a writer that provides the bytes per second by sending it through the provided `Sender`.
    #[inline]
    pub fn sender(w: impl Write, sender: Sender<Bps>) -> impl Write {
        slot(w, move |bps| {
            let _ = sender.send(bps);
        })
    }

    /// Returns a writer that provides the bytes per second by calling the provided function.
    #[inline]
    pub fn slot(w: impl Write, f: impl FnMut(Bps)) -> impl Write {
        Writer {
            w,
            f,
            bytes: 0,
            instant: Instant::now(),
        }
    }
}
