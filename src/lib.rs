//! Provides functionality for finding the throughput of iterators, readers, and writers.
//!
//! # Examples
//!
//! Add this to `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! nyx = "0.1"
//! ```
//!
//! And this to `main.rs`:
//!
//! ```no_run
//! use std::io;
//!
//! fn main() {
//!     io::copy(&mut nyx::read::stdout(io::repeat(0)), &mut io::sink()).unwrap();
//! }
//! ```
//!
//! This will write the amount of bytes copied per second to `stdout` in one second intervals.
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

use std::cell::Cell;
use std::fmt::{self, Display, Formatter};
use std::time::{Duration, Instant};

thread_local!(static INTERVAL: Cell<Duration> = Cell::new(Duration::from_secs(1)));

/// Gets the update interval for the current thread.
#[inline]
pub fn get() -> Duration {
    INTERVAL.with(Cell::get)
}

/// Sets the update interval for the current thread.
///
/// By default the interval is one second.
#[inline]
pub fn set(interval: Duration) {
    INTERVAL.with(move |c| c.set(interval));
}

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
fn bytes_per_second(new: usize, sum: &mut u64, instant: &mut Instant, mut f: impl FnMut(Bps)) {
    *sum += new as u64;
    let elapsed = instant.elapsed();
    if elapsed >= get() {
        *instant = Instant::now();
        f(Bps((*sum as f64 / elapsed.as_secs_f64()) as u64));
        *sum = 0;
    }
}

/// Adapter functions for iterators.
///
/// The functions maps the input iterator and extends it with the ability to report their
/// throughput every second to the specified receiver.
pub mod iter {
    use crate::Bps;
    use std::iter::Map;
    use std::mem;
    use std::sync::mpsc::Sender;
    use std::time::Instant;

    /// Creates an iterator that yields the bytes by printing it to `stdout`.
    ///
    /// # Examples
    /// ```no_run
    /// use std::iter;
    ///
    /// fn main() {
    ///     nyx::iter::stdout(iter::repeat(0)).for_each(|_| ());
    /// }
    /// ```
    #[inline]
    pub fn stdout<I, T>(
        iter: impl IntoIterator<Item = T, IntoIter = I>,
    ) -> Map<I, impl FnMut(T) -> T>
    where
        I: Iterator<Item = T>,
    {
        slot(iter, |bps| println!("{}", bps))
    }

    /// Creates an iterator that yields the bytes by printing it to `stderr`.
    ///
    /// # Examples
    /// ```no_run
    /// use std::iter;
    ///
    /// fn main() {
    ///     nyx::iter::stderr(iter::repeat(0)).for_each(|_| ());
    /// }
    /// ```
    #[inline]
    pub fn stderr<I, T>(
        iter: impl IntoIterator<Item = T, IntoIter = I>,
    ) -> Map<I, impl FnMut(T) -> T>
    where
        I: Iterator<Item = T>,
    {
        slot(iter, |bps| eprintln!("{}", bps))
    }

    /// Creates an iterator that yields the bytes by sending it through the provided `Sender`.
    ///
    /// # Examples
    /// ```no_run
    /// use std::sync::mpsc;
    /// use std::thread;
    /// use std::iter;
    ///
    /// fn main() {
    ///     let (sender, receiver) = mpsc::channel();
    ///     thread::spawn(move || {
    ///         nyx::iter::send(iter::repeat(0), sender).for_each(|_| ());
    ///     });
    ///     receiver
    ///         .iter()
    ///         .for_each(|bps| println!("B/s from thread: {}", bps));
    /// }
    /// ```
    #[inline]
    pub fn send<I, T>(
        iter: impl IntoIterator<Item = T, IntoIter = I>,
        sender: Sender<Bps>,
    ) -> Map<I, impl FnMut(T) -> T>
    where
        I: Iterator<Item = T>,
    {
        slot(iter, move |bps| {
            let _ = sender.send(bps);
        })
    }

    /// Creates an iterator that yields the bytes by calling the provided slot.
    ///
    /// # Examples
    /// ```no_run
    /// use std::iter;
    ///
    /// fn main() {
    ///     nyx::iter::slot(iter::repeat(0), |bps| println!("B/s: {}", bps)).for_each(|_| ());
    /// }
    /// ```
    #[inline]
    pub fn slot<I, T>(
        iter: impl IntoIterator<Item = T, IntoIter = I>,
        mut slot: impl FnMut(Bps),
    ) -> Map<I, impl FnMut(T) -> T>
    where
        I: Iterator<Item = T>,
    {
        let mut bytes = 0;
        let mut instant = Instant::now();
        iter.into_iter().map(move |item| {
            crate::bytes_per_second(mem::size_of_val(&item), &mut bytes, &mut instant, &mut slot);
            item
        })
    }
}

/// Adapter functions for readers.
///
/// The functions returns a new reader that extends the `read` and `read_vectored`
/// implementations to be able to report their throughput every second.
/// If any other methods on the reader has been specialized to not use one of the above methods,
/// this reader will not report anything.
pub mod read {
    use crate::Bps;
    use std::io::{self, IoSliceMut, Read};
    use std::sync::mpsc::Sender;
    use std::time::Instant;

    /// A reader that extends the `read` and `read_vectored` implementations to
    /// report their throughput every second.
    #[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
    struct Reader<R, F> {
        reader: R,
        slot: F,
        bytes: u64,
        instant: Instant,
    }

    impl<R: Read, F: FnMut(Bps)> Read for Reader<R, F> {
        #[inline]
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let bytes = self.reader.read(buf)?;
            crate::bytes_per_second(bytes, &mut self.bytes, &mut self.instant, &mut self.slot);
            Ok(bytes)
        }

        #[inline]
        fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
            let bytes = self.reader.read_vectored(bufs)?;
            crate::bytes_per_second(bytes, &mut self.bytes, &mut self.instant, &mut self.slot);
            Ok(bytes)
        }
    }

    /// Creates a reader that yields the bytes by printing it to `stdout`.
    ///
    /// # Examples
    /// ```no_run
    /// use std::io;
    ///
    /// fn main() {
    ///     io::copy(&mut nyx::read::stdout(io::repeat(0)), &mut io::sink()).unwrap();
    /// }
    /// ```
    #[inline]
    pub fn stdout(reader: impl Read) -> impl Read {
        slot(reader, |bps| println!("{}", bps))
    }

    /// Creates a reader that yields the bytes by printing it to `stderr`.
    ///
    /// # Examples
    /// ```no_run
    /// use std::io;
    ///
    /// fn main() {
    ///     io::copy(&mut nyx::read::stderr(io::repeat(0)), &mut io::sink()).unwrap();
    /// }
    /// ```
    #[inline]
    pub fn stderr(reader: impl Read) -> impl Read {
        slot(reader, |bps| eprintln!("{}", bps))
    }

    /// Creates a reader that yields the bytes by sending it through the provided `Sender`.
    ///
    /// # Examples
    /// ```no_run
    /// use std::sync::mpsc;
    /// use std::thread;
    /// use std::io;
    ///
    /// fn main() {
    ///     let (sender, receiver) = mpsc::channel();
    ///     thread::spawn(move || {
    ///         io::copy(
    ///             &mut nyx::read::send(io::repeat(0), sender),
    ///             &mut io::sink(),
    ///         )
    ///         .unwrap();
    ///     });
    ///     receiver
    ///         .iter()
    ///         .for_each(|bps| println!("B/s from thread: {}", bps));
    /// }
    /// ```
    #[inline]
    pub fn send(reader: impl Read, sender: Sender<Bps>) -> impl Read {
        slot(reader, move |bps| {
            let _ = sender.send(bps);
        })
    }

    /// Creates a reader that yields the bytes by calling the provided slot.
    ///
    /// # Examples
    /// ```no_run
    /// use std::io;
    ///
    /// fn main() {
    ///     io::copy(
    ///         &mut nyx::read::slot(io::repeat(0), |bps| println!("B/s: {}", bps)),
    ///         &mut io::sink(),
    ///     )
    ///     .unwrap();
    /// }
    /// ```
    #[inline]
    pub fn slot(reader: impl Read, slot: impl FnMut(Bps)) -> impl Read {
        Reader {
            reader,
            slot,
            bytes: 0,
            instant: Instant::now(),
        }
    }
}

/// Adapter functions for writers.
///
/// The functions returns a new writer that extends the `write` and `write_vectored`
/// implementations to be able to report their throughput every second.
/// If any other methods on the writer has been specialized to not use one of the above methods,
/// this writer will not report anything.
pub mod write {
    use crate::Bps;
    use std::io::{self, IoSlice, Write};
    use std::sync::mpsc::Sender;
    use std::time::Instant;

    /// A writer that extends the `write` and `write_vectored` implementations to be able to
    /// report their throughput every second.
    #[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
    struct Writer<W, F> {
        writer: W,
        slot: F,
        bytes: u64,
        instant: Instant,
    }

    impl<W: Write, F: FnMut(Bps)> Write for Writer<W, F> {
        #[inline]
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let bytes = self.writer.write(buf)?;
            crate::bytes_per_second(bytes, &mut self.bytes, &mut self.instant, &mut self.slot);
            Ok(bytes)
        }

        #[inline]
        fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
            let bytes = self.writer.write_vectored(bufs)?;
            crate::bytes_per_second(bytes, &mut self.bytes, &mut self.instant, &mut self.slot);
            Ok(bytes)
        }

        #[inline]
        fn flush(&mut self) -> io::Result<()> {
            self.writer.flush()
        }
    }

    /// Creates a writer that yields the bytes by printing it to `stdout`.
    ///
    /// # Examples
    /// ```no_run
    /// use std::io;
    ///
    /// fn main() {
    ///     io::copy(&mut io::repeat(0), &mut nyx::write::stdout(io::sink())).unwrap();
    /// }
    /// ```
    #[inline]
    pub fn stdout(writer: impl Write) -> impl Write {
        slot(writer, |bps| println!("{}", bps))
    }

    /// Creates a writer that yields the bytes by printing it to `stderr`.
    ///
    /// # Examples
    /// ```no_run
    /// use std::io;
    ///
    /// fn main() {
    ///     io::copy(&mut io::repeat(0), &mut nyx::write::stderr(io::sink())).unwrap();
    /// }
    /// ```
    #[inline]
    pub fn stderr(writer: impl Write) -> impl Write {
        slot(writer, |bps| eprintln!("{}", bps))
    }

    /// Creates a writer that yields the bytes by sending it through the provided `Sender`.
    ///
    /// # Examples
    /// ```no_run
    /// use std::sync::mpsc;
    /// use std::thread;
    /// use std::io;
    ///
    /// fn main() {
    ///     let (sender, receiver) = mpsc::channel();
    ///     thread::spawn(move || {
    ///         io::copy(
    ///             &mut io::repeat(0),
    ///             &mut nyx::write::send(io::sink(), sender),
    ///         )
    ///         .unwrap();
    ///     });
    ///     receiver
    ///         .iter()
    ///         .for_each(|bps| println!("B/s from thread: {}", bps));
    /// }
    /// ```
    #[inline]
    pub fn send(writer: impl Write, sender: Sender<Bps>) -> impl Write {
        slot(writer, move |bps| {
            let _ = sender.send(bps);
        })
    }

    /// Creates a writer that yields the bytes by calling the provided slot.
    ///
    /// # Examples
    /// ```no_run
    /// use std::io;
    ///
    /// fn main() {
    ///     io::copy(
    ///         &mut io::repeat(0),
    ///         &mut nyx::write::slot(io::sink(), |bps| println!("B/s: {}", bps)),
    ///     )
    ///     .unwrap();
    /// }
    /// ```
    #[inline]
    pub fn slot(writer: impl Write, slot: impl FnMut(Bps)) -> impl Write {
        Writer {
            writer,
            slot,
            bytes: 0,
            instant: Instant::now(),
        }
    }
}
