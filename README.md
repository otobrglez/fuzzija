# fuzzija

Fuzzija is a fast search engine for public Slovenian tax data.

## Usage && Development

CLI usage with `fuzzija` binary.

```
Usage: fuzzija [OPTIONS]

Options:
      --storage-folder <STORAGE_FOLDER>  [env: STORAGE_FOLDER=] [default: tmp]
      --indexes-folder <INDEXES_FOLDER>  [env: INDEXES_FOLDER=] [default: indexes]
      --force-download
  -r, --reindex
  -q, --query <QUERY>
  -h, --help                             Print help
  -V, --version                          Print version
```

The `fuzzija-server` with:

```
Usage: fuzzija-server [OPTIONS]

Options:
      --storage-folder <STORAGE_FOLDER>  [env: STORAGE_FOLDER=] [default: tmp]
      --indexes-folder <INDEXES_FOLDER>  [env: INDEXES_FOLDER=] [default: indexes]
      --force-download
  -r, --reindex
  -q, --query <QUERY>
  -p, --port <PORT>                      [env: PORT=] [default: 8080]
      --host <HOST>                      [default: 0.0.0.0]
  -h, --help                             Print help
  -V, --version                          Print version
````

```bash
RUST_LOG=info cargo run -- --force-download --reindex # Download and (re)index the data
RUST_LOG=info cargo run -- --query "opalab"  # Query all the indexes

# or build...
cargo build --relase

./target/release/fuzzija -h
```

\- Oto Brglez
