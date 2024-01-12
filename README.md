Full documentation: [https://docs.rs/librsv/0.1.0/librsv/](https://docs.rs/librsv/0.1.0/librsv/)

RSV (Rows of String Values) is a very simple binary format for encoding tabular data.
It is similar to CSV, but even simpler due to the avoidance of escape characters.
This is achieved by encoding strings as UTF-8, and using bytes that can never appear in valid UTF-8 strings as delimiters.

The full specification can be found at: [https://github.com/Stenway/RSV-Specification](https://github.com/Stenway/RSV-Specification)

# Basic usage

There are three convenience methods for encoding and decoding RSV documents in one go:

- `encode_rsv` - Encodes an RSV document from a structure such as `Vec<Vec<Option<String>>>`.
- `decode_rsv`- Decodes an RSV document into a `Vec<Vec<Option<String>>>`.
- `decode_rsv_borrowed`- Decodes an RSV document into a `Vec<Vec<Option<&str>>>`.

```
use librsv::{encode_rsv, decode_rsv};

let data = vec![
    vec![Some("Hello".into()), Some("world".into())],
    vec![Some("asdf".into()), None, Some("".into())],
];

let encoded = encode_rsv(&data);
let decoded = decode_rsv(&encoded).unwrap();

assert_eq!(data, decoded);
```
