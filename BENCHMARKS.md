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
- SHA-256 compresses full 64-byte data blocks directly and only uses padded
  reads for the final block(s)

Criterion results:

| Benchmark | Mean time | Throughput | Speedup |
| --- | ---: | ---: | ---: |
| `twox_hash_xxhash32_cpu` | 67.650 ms | 7.3910 GiB/s | 1.0x |
| `gpuhash_xxhash32_prepared` | 3.0395 ms | 164.50 GiB/s | 22.26x |
| `twox_hash_xxhash64_cpu` | 20.650 ms | 24.213 GiB/s | 1.0x |
| `gpuhash_xxhash64_prepared` | 3.0652 ms | 163.12 GiB/s | 6.74x |
| `twox_hash_xxhash3_64_cpu` | 11.197 ms | 44.654 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_prepared` | 3.2339 ms | 154.61 GiB/s | 3.46x |
| `twox_hash_xxhash3_128_cpu` | 11.687 ms | 42.782 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_prepared` | 3.2698 ms | 152.91 GiB/s | 3.57x |
| `twox_hash_xxhash3_64_secret_cpu` | 11.500 ms | 43.480 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_secret_prepared` | 3.2626 ms | 153.25 GiB/s | 3.52x |
| `twox_hash_xxhash3_128_secret_cpu` | 12.271 ms | 40.745 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_secret_prepared` | 3.2812 ms | 152.38 GiB/s | 3.74x |
| `rustcrypto_sha256_cpu` | 165.19 ms | 3.0269 GiB/s | 1.0x |
| `gpuhash_sha256_prepared` | 9.7215 ms | 51.433 GiB/s | 16.99x |

The quick benchmark is useful for a one-shot sanity check:

```sh
cargo run --release --example quick_bench
```

It also accepts optional `count len` arguments:

```sh
cargo run --release --example quick_bench -- 131072 4096
```
