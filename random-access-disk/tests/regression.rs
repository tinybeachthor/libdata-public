use random_access_disk as rad;
use random_access_storage::RandomAccess;
use std::env;
use tempfile::Builder;

#[async_std::test]
// postmortem: read_exact wasn't behaving like we hoped,
// switch back to `.read()` and disable clippy for that rule specifically.
pub async fn regress_1() {
  let dir = Builder::new()
    .prefix("random-access-disk")
    .tempdir()
    .unwrap();
  let mut file =
    rad::RandomAccessDisk::open(dir.path().join("regression-1.db"))
      .await
      .unwrap();
  file.write(27, b"").await.unwrap();
  file.read(13, 5).await.unwrap();
}

#[async_std::test]
// postmortem: accessing the same file twice would fail,
// switch to from `.create_new()` to `.create()`.
pub async fn regress_2() {
    regress_2_helper().await;
    regress_2_helper().await;
}

async fn regress_2_helper() {
  let mut dir = env::temp_dir();
  dir.push("regression-2.db");
  let mut file = rad::RandomAccessDisk::open(dir).await.unwrap();
  file.write(27, b"").await.unwrap();
}
