mod common;
use common::{
    create_duplex_pair_memory,
    create_pair_memory_keepalive,
    establish,
};

use anyhow::Result;
use std::mem::drop;
use std::time::Duration;
use std::task::{Poll, Context};
use async_std::test;
use async_std::task::sleep;
use futures_lite::stream::StreamExt;
use futures_test::task::noop_waker;

use protocol::{Options, new_protocol, main::Event};

#[test]
async fn timeout_no_connection() -> Result<()>
{
    let keepalive_ms = 100;
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    let (a, b) = create_duplex_pair_memory();
    let mut proto_a = new_protocol(a, Options {
        is_initiator: true,
        keepalive_ms: Some(keepalive_ms),
        ..Options::default()
    });

    assert!(matches!(proto_a.poll_next(&mut cx), Poll::Pending));
    sleep(Duration::from_millis(60)).await;
    assert!(matches!(proto_a.poll_next(&mut cx), Poll::Pending));
    sleep(Duration::from_millis(keepalive_ms)).await;
    assert!(matches!(proto_a.poll_next(&mut cx), Poll::Ready(Some(Err(_)))));

    drop(b);
    Ok(())
}

#[test]
async fn timeout_reset_on_handshake() -> Result<()> {
    let keepalive_ms = 100;
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    let (mut proto_a, proto_b) =
        create_pair_memory_keepalive(Some(keepalive_ms))?;

    assert!(matches!(proto_a.poll_next(&mut cx), Poll::Pending));
    sleep(Duration::from_millis(keepalive_ms - 40)).await;

    let (mut proto_a, proto_b) = establish(proto_a, proto_b).await;

    sleep(Duration::from_millis(keepalive_ms - 40)).await;
    assert!(matches!(proto_a.poll_next(&mut cx), Poll::Pending));
    sleep(Duration::from_millis(keepalive_ms)).await;
    assert!(matches!(proto_a.poll_next(&mut cx), Poll::Ready(Some(Err(_)))));

    drop(proto_b);
    Ok(())
}

#[test]
async fn timeout_reading_resets_timeout_writing_not() -> Result<()> {
    let keepalive_ms = 100;
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let key = [3u8; 32];

    let (mut proto_a, proto_b) =
        create_pair_memory_keepalive(Some(keepalive_ms))?;

    assert!(matches!(proto_a.poll_next(&mut cx), Poll::Pending));
    sleep(Duration::from_millis(keepalive_ms - 40)).await;

    let (mut proto_a, mut proto_b) = establish(proto_a, proto_b).await;

    sleep(Duration::from_millis(30)).await;
    proto_a.open(key.clone()).await?;
    assert!(matches!(proto_a.poll_next(&mut cx), Poll::Pending));
    assert!(matches!(proto_b.poll_next(&mut cx), Poll::Ready(Some(
                    Ok(Event::DiscoveryKey(_))))));

    sleep(Duration::from_millis(30)).await;
    proto_a.open(key.clone()).await?;
    assert!(matches!(proto_a.poll_next(&mut cx), Poll::Pending));
    assert!(matches!(proto_b.poll_next(&mut cx), Poll::Ready(Some(
                    Ok(Event::DiscoveryKey(_))))));

    sleep(Duration::from_millis(60)).await;
    assert!(matches!(proto_a.poll_next(&mut cx), Poll::Ready(Some(Err(_)))));
    assert!(matches!(proto_b.poll_next(&mut cx), Poll::Pending));

    sleep(Duration::from_millis(60)).await;
    assert!(matches!(proto_b.poll_next(&mut cx), Poll::Ready(Some(Err(_)))));

    Ok(())
}
