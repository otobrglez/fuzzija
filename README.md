# fuzzija

Fuzzija is a fast search engine for public Slovenian tax data.

## Usage && Development

```
Usage: fuzzija [OPTIONS]

Options:
  -s, --storage-folder <STORAGE_FOLDER>  [default: tmp]
  -i, --indexes-folder <INDEXES_FOLDER>  [default: indexes]
      --force-download
      --reindex
      --query <QUERY>
  -h, --help                             Print help
  -V, --version                          Print version
```

```bash
RUST_LOG=info cargo run -- --force-download --reindex # Download and (re)index the data
RUST_LOG=info cargo run -- --query "opalab"  # Query all the indexes
# or build...
cargo build --relase
./target/release/fuzzija -h
```

\- Oto Brglez
