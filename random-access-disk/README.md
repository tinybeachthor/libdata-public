# random-access-disk

Continuously read/write to disk, using random offsets and lengths. Adapted from
[random-access-storage/random-access-file](https://github.com/random-access-storage/random-access-file/).

## Usage
```rust
use std::path::PathBuf;
use tempdir::TempDir;

let dir = TempDir::new("random-access-disk").unwrap();
let mut file = random_access_disk::RandomAccessDisk::new(dir.path().join("README.db"));

file.write(0, b"hello").await.unwrap();
file.write(5, b" world").await.unwrap();
let _text = file.read(0, 11).await.unwrap();
```

## License
[MIT](./LICENSE-MIT) OR [Apache-2.0](./LICENSE-APACHE)
