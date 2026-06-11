# Benchmarks

Machine:

- Apple M5 Pro
- macOS 26.5.1
- Rust 1.95.0

Command:

```sh
cargo bench --bench hash_batch
```

The same benchmark runs in the
[GitHub Actions benchmark job](https://github.com/wan9chi/gpuhash/actions/workflows/ci.yml?query=branch%3Amain).
Each run uploads the Criterion report as the `criterion-report` artifact.

Benchmark batch:

- 131,072 messages
- 4,096 bytes per message
- 512 MiB total input
- custom-secret XXH3 rows use a 257-byte secret
- GPU data path uses `PreparedBatch` shared Metal buffers
- GPU output buffers wrap the returned Rust `Vec` allocation directly with
  `newBufferWithBytesNoCopy`

Criterion results:

| Benchmark | Mean time | Throughput | Speedup |
| --- | ---: | ---: | ---: |
| `twox_hash_xxhash32_cpu` | 68.158 ms | 7.3359 GiB/s | 1.0x |
| `gpuhash_xxhash32_prepared` | 3.0392 ms | 164.52 GiB/s | 22.43x |
| `twox_hash_xxhash64_cpu` | 20.656 ms | 24.206 GiB/s | 1.0x |
| `gpuhash_xxhash64_prepared` | 3.0609 ms | 163.35 GiB/s | 6.75x |
| `twox_hash_xxhash3_64_cpu` | 11.533 ms | 43.354 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_prepared` | 3.1916 ms | 156.66 GiB/s | 3.61x |
| `twox_hash_xxhash3_128_cpu` | 11.687 ms | 42.783 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_prepared` | 3.2612 ms | 153.32 GiB/s | 3.58x |
| `twox_hash_xxhash3_64_secret_cpu` | 11.860 ms | 42.158 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_secret_prepared` | 3.2470 ms | 153.99 GiB/s | 3.65x |
| `twox_hash_xxhash3_128_secret_cpu` | 11.968 ms | 41.777 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_secret_prepared` | 3.2671 ms | 153.04 GiB/s | 3.66x |
| `rustcrypto_sha256_cpu` | 165.42 ms | 3.0226 GiB/s | 1.0x |
| `gpuhash_sha256_prepared` | 12.576 ms | 39.759 GiB/s | 13.15x |

The quick benchmark is useful for a one-shot sanity check:

```sh
cargo run --release --example quick_bench
```

It also accepts optional `count len` arguments:

```sh
cargo run --release --example quick_bench -- 131072 4096
```
