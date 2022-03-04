use random_access_disk as rad;
use random_access_storage::RandomAccess;
use tempfile::Builder;

#[async_std::test]
async fn can_call_new() {
  let dir = Builder::new()
    .prefix("random-access-disk")
    .tempdir()
    .unwrap();
  let _file = rad::RandomAccessDisk::open(dir.path().join("1.db"))
    .await
    .unwrap();
}

#[async_std::test]
async fn can_open_buffer() {
  let dir = Builder::new()
    .prefix("random-access-disk")
    .tempdir()
    .unwrap();
  let mut file = rad::RandomAccessDisk::open(dir.path().join("2.db"))
    .await
    .unwrap();
  file.write(0, b"hello").await.unwrap();
}

#[async_std::test]
async fn can_write() {
  let dir = Builder::new()
    .prefix("random-access-disk")
    .tempdir()
    .unwrap();
  let mut file = rad::RandomAccessDisk::open(dir.path().join("3.db"))
    .await
    .unwrap();
  file.write(0, b"hello").await.unwrap();
  file.write(5, b" world").await.unwrap();
}

#[async_std::test]
async fn can_read() {
  let dir = Builder::new()
    .prefix("random-access-disk")
    .tempdir()
    .unwrap();
  let mut file = rad::RandomAccessDisk::open(dir.path().join("4.db"))
    .await
    .unwrap();
  file.write(0, b"hello").await.unwrap();
  file.write(5, b" world").await.unwrap();
  let text = file.read(0, 11).await.unwrap();
  assert_eq!(String::from_utf8(text.to_vec()).unwrap(), "hello world");
}
