//! RSV (Rows of String Values) is a very simple binary format for encoding tabular data.
//! It is similar to CSV, but even simpler due to the avoidance of escape characters.
//! This is achieved by encoding strings as UTF-8, and using bytes that can never appear in valid UTF-8 strings as delimiters.
//!
//! The full specification can be found at: [https://github.com/Stenway/RSV-Specification](https://github.com/Stenway/RSV-Specification)
//!
//! # Basic usage
//!
//! There are three convenience methods for encoding and decoding RSV documents in one go:
//! * `encode_rsv` - Encodes an RSV document from a structure such as `Vec<Vec<Option<String>>>`.
//! * `decode_rsv`- Decodes an RSV document into a `Vec<Vec<Option<String>>>`.
//! * `decode_rsv_borrowed`- Decodes an RSV document into a `Vec<Vec<Option<&str>>>`.
//!
//! ```
//! use librsv::{encode_rsv, decode_rsv};
//!
//! let data = vec![
//!     vec![Some("Hello".into()), Some("world".into())],
//!     vec![Some("asdf".into()), None, Some("".into())],
//! ];
//!
//! let encoded = encode_rsv(&data);
//! let decoded = decode_rsv(&encoded).unwrap();
//!
//! assert_eq!(data, decoded);
//! ```
//!
//! # Advanced usage
//!
//! For more control, there exists the `RsvWriter` and `RsvReader` structs which
//! allow for more control over how the data is encoded or decoded respectively:
//!
//! ```
//! use librsv::{RsvReader, RsvWriter};
//!
//! /// Write an RSV document
//! let mut writer = RsvWriter::new();
//! writer.start_row();
//! writer.push_str("Hello");
//! writer.push_null();
//! let buffer = writer.finish();
//!
//! /// Read an RSV document and prints its contents to the screen
//! let mut reader = RsvReader::new(&buffer);
//! for row in reader.rows() {
//!     let row = row.unwrap();
//!     for value in row.values() {
//!         match value.unwrap() {
//!             Some(str) => print!("\"{str}\", "),
//!             None => print!("null, ")
//!         }
//!         println!();
//!     }
//! }
//! ```

use thiserror::Error;

/// Row termination byte.
const END_ROW: u8 = 0xFD;
/// Represents an absent value.
const NULL_VALUE: u8 = 0xFE;
/// Value termination byte.
const END_VALUE: u8 = 0xFF;

/// An error encountered while parsing an RSV stream.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The input ended without a row terminator byte.
    #[error("unexpected end of input, expected a row terminator")]
    UnterminatedRow,
    /// The row ended without a value terminator byte.
    #[error("unexpected end of row, expected a value terminator")]
    UnterminatedValue,
    /// A value contained invalid UTF-8.
    #[error("a value contained invalid UTF-8: {0}")]
    BadUTF8(std::str::Utf8Error),
}

/// A convenience method for encoding an RSV document.
///
/// The generic parameters allow for encoding a variety of owned or borrowed data structures, such as:
/// * `Vec<Vec<Option<String>>>`
/// * `Vec<Vec<Option<str>>>`
/// * `Vec<&[Option<str>]>`
/// * `&[&[Option<str>]]`
///
/// # Example:
/// ```
/// let buffer = librsv::encode_rsv(vec![
///     vec![Some("Hello"), Some("world")],
///     vec![None, Some("asdf")]
/// ]);
///
/// assert_eq!(&buffer, b"Hello\xFFworld\xFF\xFD\xFE\xFFasdf\xFF\xFD");
/// ```
pub fn encode_rsv<T, R, V>(rows: T) -> Vec<u8>
where
    T: AsRef<[R]>,
    R: AsRef<[Option<V>]>,
    V: AsRef<str>,
{
    let mut writer = RsvWriter::new();
    for row in rows.as_ref() {
        writer.start_row();
        for value in row.as_ref() {
            writer.push(value.as_ref().map(|v| v.as_ref()))
        }
    }
    writer.finish()
}

/// A convenience method for decoding an RSV document into a `Vec<Vec<Option<String>>>`.
///
/// # Example:
/// ```
/// let buffer = b"Hello\xFFworld\xFF\xFD";
/// let data = librsv::decode_rsv(buffer).unwrap();
///
/// assert_eq!(data, vec![vec![Some("Hello".into()), Some("world".into())]]);
/// ```
pub fn decode_rsv(data: &[u8]) -> Result<Vec<Vec<Option<String>>>, Error> {
    RsvReader::new(data)
        .rows()
        .map(|row| {
            row?.values()
                .map(|v| v.map(|v| v.map(|v| v.to_string())))
                .collect::<Result<_, _>>()
        })
        .collect::<Result<_, _>>()
}

/// A convenience method for decoding an RSV document into a `Vec<Vec<Option<&str>>>`,
/// with the string values borrowing from the encoded bytes.
pub fn decode_rsv_borrowed(data: &[u8]) -> Result<Vec<Vec<Option<&str>>>, Error> {
    RsvReader::new(data)
        .rows()
        .map(|row| row?.values().collect::<Result<_, _>>())
        .collect::<Result<_, _>>()
}

/// Writes an RSV document to an internal `Vec<u8>`.
#[derive(Clone, Default)]
pub struct RsvWriter {
    buffer: Vec<u8>,
    started_row: bool,
}

impl RsvWriter {
    /// Creates a new `RsvWriter`.
    pub fn new() -> Self {
        Self::with_buffer(vec![])
    }

    /// Creates a new `RsvWriter`, with a given initial capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_buffer(Vec::with_capacity(capacity))
    }

    /// Creates a new `RsvWriter`, with the given buffer.
    ///
    /// If the buffer is not empty, the new data will be appended to the existing data.
    pub fn with_buffer(buffer: Vec<u8>) -> Self {
        Self {
            buffer,
            ..Self::default()
        }
    }

    /// Begins a new row.
    ///
    /// This must be called before pushing any values.
    pub fn start_row(&mut self) {
        if self.started_row {
            self.buffer.push(END_ROW);
        }
        self.started_row = true;
    }

    /// Pushes a value to the current row.
    pub fn push(&mut self, value: Option<&str>) {
        assert!(self.started_row, "must start a row before pushing a value");
        match value {
            Some(str) => self.buffer.extend(str.as_bytes()),
            None => self.buffer.push(NULL_VALUE),
        }
        self.buffer.push(END_VALUE);
    }

    /// Pushes a string value to the current row.
    pub fn push_str(&mut self, value: &str) {
        self.push(Some(value))
    }

    /// Pushes an empty value to the current row.
    pub fn push_null(&mut self) {
        self.push(None)
    }

    /// Finishes writing and returns the inner buffer.
    pub fn finish(self) -> Vec<u8> {
        let mut buffer = self.buffer;
        if self.started_row {
            buffer.push(END_ROW);
        }
        buffer
    }
}

/// Reads an RSV document.
pub struct RsvReader<'a> {
    data: &'a [u8],
}

/// Reads an RSV row.
pub struct RsvRow<'a> {
    data: &'a [u8],
}

impl<'a> RsvReader<'a> {
    /// Creates a new `RsvReader` from the provided buffer.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Iterates over the rows in the RSV document.
    pub fn rows(&self) -> impl Iterator<Item = Result<RsvRow<'a>, Error>> {
        let mut remain = self.data;
        std::iter::from_fn(move || {
            if remain.is_empty() {
                return None;
            }
            let Some(terminator) = remain.iter().position(|c| *c == END_ROW) else {
                return Some(Err(Error::UnterminatedRow));
            };
            let (row, rest) = remain.split_at(terminator);
            remain = &rest[1..];
            Some(Ok(RsvRow::new(row)))
        })
    }
}

impl<'a> RsvRow<'a> {
    /// Creates a new `RsvRow` from the provided buffer.
    ///
    /// This generally won't be called directly.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Iterates over the values in the RSV row.
    pub fn values(&self) -> impl Iterator<Item = Result<Option<&'a str>, Error>> {
        let mut remain = self.data;
        std::iter::from_fn(move || {
            if remain.is_empty() {
                return None;
            }
            let Some(terminator) = remain.iter().position(|c| *c == END_VALUE) else {
                return Some(Err(Error::UnterminatedValue));
            };
            let (value, rest) = remain.split_at(terminator);
            remain = &rest[1..];
            match value {
                [NULL_VALUE] => Some(Ok(None)),
                bytes => Some(std::str::from_utf8(bytes).map(Some).map_err(Error::BadUTF8)),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut w = RsvWriter::new();

        // Row 1
        w.start_row();
        w.push_str("Hello");
        w.push_str("world");

        // Row 2
        w.start_row();
        w.push_str("");
        w.push_null();
        w.push_str("world 2");
        w.push_null();

        // Row 3 - empty
        w.start_row();

        let buffer = w.finish();
        let mut rows = RsvReader::new(&buffer).rows();

        // Row 1
        let mut values = rows.next().unwrap().unwrap().values();
        assert_eq!(values.next().unwrap().unwrap(), Some("Hello"));
        assert_eq!(values.next().unwrap().unwrap(), Some("world"));
        assert!(values.next().is_none());

        // Row 2
        let mut values = rows.next().unwrap().unwrap().values();
        assert_eq!(values.next().unwrap().unwrap(), Some(""));
        assert_eq!(values.next().unwrap().unwrap(), None);
        assert_eq!(values.next().unwrap().unwrap(), Some("world 2"));
        assert_eq!(values.next().unwrap().unwrap(), None);
        assert!(values.next().is_none());

        // Row 3 - empty
        let mut values = rows.next().unwrap().unwrap().values();
        assert!(values.next().is_none());

        assert!(rows.next().is_none());
    }

    #[test]
    fn encode_vec_vec_string() {
        let data: Vec<Vec<Option<String>>> =
            vec![vec![Some("Hello".into()), Some("world".into())], vec![None]];
        encode_rsv(data);
    }

    #[test]
    fn encode_vec_vec_str() {
        let data: Vec<Vec<Option<&str>>> = vec![vec![Some("Hello"), Some("world")], vec![None]];
        encode_rsv(data);
    }

    #[test]
    fn encode_vec_slice_str() {
        let values = vec![Some("Hello"), Some("world"), None];
        let data: Vec<&[Option<&str>]> = vec![&values, &values[1..]];
        encode_rsv(data);
    }

    #[test]
    fn encode_slice_slice_str() {
        let values = vec![Some("Hello"), Some("world"), None];
        let data: &[&[Option<&str>]] = &[&values, &values[1..]];
        encode_rsv(data);
    }
}
