use gpuhash::{GpuHash, GpuHashError};
use sha2::{Digest as _, Sha256};
use std::{env, hint::black_box, time::Instant};
use twox_hash::{XxHash3_64, XxHash3_128, XxHash32, XxHash64};

const DEFAULT_COUNT: usize = 131_072;
const DEFAULT_LEN: usize = 4096;
const SEED: u64 = 0x1234_5678_9abc_def0;

fn main() -> gpuhash::Result<()> {
    let count = parse_arg(1, DEFAULT_COUNT)?;
    let len = parse_arg(2, DEFAULT_LEN)?;
    println!("messages: {count}, bytes/message: {len}");

    let messages = messages(count, len);
    let gpu = GpuHash::new()?;
    let batch = gpu.prepare_batch(&messages)?;

    let gpu_xx32 = measure("gpu xxhash32 prepared", batch.total_bytes(), || {
        gpu.xxhash32_prepared(SEED as u32, &batch)
    })?;
    let cpu_xx32 = measure("cpu twox-hash xxhash32", batch.total_bytes(), || {
        Ok(xxhash32_cpu(&messages))
    })?;
    assert_eq!(gpu_xx32, cpu_xx32);

    let gpu_xx = measure("gpu xxhash64 prepared", batch.total_bytes(), || {
        gpu.xxhash64_prepared(SEED, &batch)
    })?;
    let cpu_xx = measure("cpu twox-hash xxhash64", batch.total_bytes(), || {
        Ok(xxhash64_cpu(&messages))
    })?;
    assert_eq!(gpu_xx, cpu_xx);

    let gpu_xxh3_64 = measure("gpu xxhash3-64 prepared", batch.total_bytes(), || {
        gpu.xxhash3_64_with_seed_prepared(SEED, &batch)
    })?;
    let cpu_xxh3_64 = measure("cpu twox-hash xxhash3-64", batch.total_bytes(), || {
        Ok(xxhash3_64_cpu(&messages))
    })?;
    assert_eq!(gpu_xxh3_64, cpu_xxh3_64);

    let gpu_xxh3_128 = measure("gpu xxhash3-128 prepared", batch.total_bytes(), || {
        gpu.xxhash3_128_with_seed_prepared(SEED, &batch)
    })?;
    let cpu_xxh3_128 = measure("cpu twox-hash xxhash3-128", batch.total_bytes(), || {
        Ok(xxhash3_128_cpu(&messages))
    })?;
    assert_eq!(gpu_xxh3_128, cpu_xxh3_128);

    let gpu_sha = measure("gpu sha256 prepared", batch.total_bytes(), || {
        gpu.sha256_prepared(&batch)
    })?;
    let cpu_sha = measure("cpu RustCrypto sha256", batch.total_bytes(), || {
        Ok(sha256_cpu(&messages))
    })?;
    assert_eq!(gpu_sha, cpu_sha);

    Ok(())
}

fn parse_arg(index: usize, default: usize) -> gpuhash::Result<usize> {
    match env::args().nth(index) {
        Some(value) => value
            .parse()
            .map_err(|err| GpuHashError::Metal(format!("invalid numeric argument: {err}"))),
        None => Ok(default),
    }
}

fn messages(count: usize, len: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|message_idx| {
            (0..len)
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
        .map(|message| XxHash64::oneshot(SEED, black_box(message)))
        .collect()
}

fn xxhash32_cpu(input: &[Vec<u8>]) -> Vec<u32> {
    input
        .iter()
        .map(|message| XxHash32::oneshot(SEED as u32, black_box(message)))
        .collect()
}

fn xxhash3_64_cpu(input: &[Vec<u8>]) -> Vec<u64> {
    input
        .iter()
        .map(|message| XxHash3_64::oneshot_with_seed(SEED, black_box(message)))
        .collect()
}

fn xxhash3_128_cpu(input: &[Vec<u8>]) -> Vec<u128> {
    input
        .iter()
        .map(|message| XxHash3_128::oneshot_with_seed(SEED, black_box(message)))
        .collect()
}

fn sha256_cpu(input: &[Vec<u8>]) -> Vec<[u8; 32]> {
    input
        .iter()
        .map(|message| Sha256::digest(black_box(message)).into())
        .collect()
}

fn measure<T>(
    name: &str,
    total_bytes: usize,
    mut f: impl FnMut() -> gpuhash::Result<T>,
) -> gpuhash::Result<T> {
    let started = Instant::now();
    let value = f()?;
    let elapsed = started.elapsed();
    let gbps = total_bytes as f64 / elapsed.as_secs_f64() / 1_000_000_000.0;
    println!(
        "{name:28} {:>8.3} ms  {:>8.2} GB/s",
        elapsed.as_secs_f64() * 1000.0,
        gbps
    );
    Ok(value)
}
