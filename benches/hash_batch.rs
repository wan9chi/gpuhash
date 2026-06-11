use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use gpuhash::GpuHash;
use sha2::{Digest as _, Sha256};
use std::hint::black_box;
use twox_hash::{XxHash3_64, XxHash3_128, XxHash32, XxHash64};

const COUNT: usize = 131_072;
const LEN: usize = 4096;
const SEED: u64 = 0x1234_5678_9abc_def0;

fn messages() -> Vec<Vec<u8>> {
    (0..COUNT)
        .map(|message_idx| {
            (0..LEN)
                .map(|byte_idx| {
                    (message_idx as u8)
                        .wrapping_mul(31)
                        .wrapping_add(byte_idx as u8)
                        .rotate_left((byte_idx & 7) as u32)
                })
                .collect()
        })
        .collect()
}

fn xxhash64_cpu(input: &[Vec<u8>]) -> Vec<u64> {
    input
        .iter()
        .map(|message| XxHash64::oneshot(SEED, message))
        .collect()
}

fn xxhash32_cpu(input: &[Vec<u8>]) -> Vec<u32> {
    input
        .iter()
        .map(|message| XxHash32::oneshot(SEED as u32, message))
        .collect()
}

fn xxhash3_64_cpu(input: &[Vec<u8>]) -> Vec<u64> {
    input
        .iter()
        .map(|message| XxHash3_64::oneshot_with_seed(SEED, message))
        .collect()
}

fn xxhash3_128_cpu(input: &[Vec<u8>]) -> Vec<u128> {
    input
        .iter()
        .map(|message| XxHash3_128::oneshot_with_seed(SEED, message))
        .collect()
}

fn sha256_cpu(input: &[Vec<u8>]) -> Vec<[u8; 32]> {
    input
        .iter()
        .map(|message| Sha256::digest(message).into())
        .collect()
}

fn bench_hashes(c: &mut Criterion) {
    let messages = messages();
    let gpu = GpuHash::new().expect("Metal GPU is required for this benchmark");
    let prepared = gpu.prepare_batch(&messages).unwrap();

    let mut group = c.benchmark_group("batch_hash");
    group.sample_size(10);
    group.throughput(Throughput::Bytes((COUNT * LEN) as u64));

    group.bench_function("twox_hash_xxhash64_cpu", |b| {
        b.iter(|| black_box(xxhash64_cpu(black_box(&messages))));
    });

    group.bench_function("twox_hash_xxhash32_cpu", |b| {
        b.iter(|| black_box(xxhash32_cpu(black_box(&messages))));
    });

    group.bench_function("gpuhash_xxhash32_prepared", |b| {
        b.iter(|| {
            black_box(
                gpu.xxhash32_prepared(SEED as u32, black_box(&prepared))
                    .unwrap(),
            )
        });
    });

    group.bench_function("gpuhash_xxhash64_prepared", |b| {
        b.iter(|| black_box(gpu.xxhash64_prepared(SEED, black_box(&prepared)).unwrap()));
    });

    group.bench_function("twox_hash_xxhash3_64_cpu", |b| {
        b.iter(|| black_box(xxhash3_64_cpu(black_box(&messages))));
    });

    group.bench_function("gpuhash_xxhash3_64_prepared", |b| {
        b.iter(|| {
            black_box(
                gpu.xxhash3_64_with_seed_prepared(SEED, black_box(&prepared))
                    .unwrap(),
            )
        });
    });

    group.bench_function("twox_hash_xxhash3_128_cpu", |b| {
        b.iter(|| black_box(xxhash3_128_cpu(black_box(&messages))));
    });

    group.bench_function("gpuhash_xxhash3_128_prepared", |b| {
        b.iter(|| {
            black_box(
                gpu.xxhash3_128_with_seed_prepared(SEED, black_box(&prepared))
                    .unwrap(),
            )
        });
    });

    group.bench_function("rustcrypto_sha256_cpu", |b| {
        b.iter(|| black_box(sha256_cpu(black_box(&messages))));
    });

    group.bench_function("gpuhash_sha256_prepared", |b| {
        b.iter(|| black_box(gpu.sha256_prepared(black_box(&prepared)).unwrap()));
    });

    group.finish();
}

criterion_group!(benches, bench_hashes);
criterion_main!(benches);
