use criterion::{black_box, criterion_group, criterion_main, Criterion};
use criterion::async_executor::AsyncStdExecutor;
use async_std::task::block_on;

use random_access_memory::RandomAccessMemory;
use datacore::{generate_keypair, Core};

type HomogenousCore<T> = Core<T, T, T>;
type MemoryCore = HomogenousCore<RandomAccessMemory>;

fn random_access_memory() -> RandomAccessMemory {
    RandomAccessMemory::new(1024)
}

fn init() -> MemoryCore {
    block_on(async move {
        let keypair = generate_keypair();
        Core::new(
            random_access_memory(),
            random_access_memory(),
            random_access_memory(),
            keypair.public, Some(keypair.secret))
            .await.unwrap()
    })
}

async fn hypercore_append(mut core: MemoryCore, blocks: u64) {
    for i in 0..blocks {
        core.append(&i.to_be_bytes(), None).await.unwrap();
    }
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("append 1000 blocks", |b| {
        b.to_async(AsyncStdExecutor).iter(|| {
            let feed = init();
            hypercore_append(black_box(feed), black_box(1_000))
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
