# random-access-storage

Abstract interface to implement random-access instances.

## Why?
This module forms a shared interface for reading and writing bytes to
different backends. By having a shared interface, it means implementations
can easily be swapped, depending on the environment.

## Usage
```rust
use random_access_storage::{RandomAccessMethods, RandomAccess};
use async_trait::async_trait;

struct S;
#[async_trait]
impl RandomAccessMethods for S {
  type Error = std::io::Error;

  async fn open(&mut self) -> Result<(), Self::Error> {
    unimplemented!();
  }

  async fn write(&mut self, offset: u64, data: &[u8]) -> Result<(), Self::Error> {
    unimplemented!();
  }

  async fn read(&mut self, offset: u64, length: u64) -> Result<Vec<u8>, Self::Error> {
    unimplemented!();
  }

  async fn read_to_writer(
    &mut self,
    offset: u64,
    length: u64,
    writer: &mut (impl futures_lite::io::AsyncWriter + Send)
  ) -> Result<(), Self::Error> {
    unimplemented!();
  }

  async fn del(&mut self, offset: u64, length: u64) -> Result<(), Self::Error> {
    unimplemented!();
  }

  async fn truncate(&mut self, length: u64) -> Result<(), Self::Error> {
    unimplemented!();
  }

  async fn len(&mut self) -> Result<u64, Self::Error> {
    unimplemented!();
  }

  async fn is_empty(&mut self) -> Result<bool, Self::Error> {
    unimplemented!();
  }

  async fn sync_all(&mut self) -> Result<(), Self::Error> {
    unimplemented!();
  }
}

let _file = RandomAccess::new(S);
```

## See Also
- [random-access-storage/random-access-storage](https://github.com/random-access-storage/random-access-storage)

## License
[MIT](./LICENSE-MIT) OR [Apache-2.0](./LICENSE-APACHE)
