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
| `twox_hash_xxhash32_cpu` | 66.061 ms | 7.5688 GiB/s | 1.0x |
| `gpuhash_xxhash32_prepared` | 3.0584 ms | 163.49 GiB/s | 21.60x |
| `twox_hash_xxhash64_cpu` | 20.409 ms | 24.499 GiB/s | 1.0x |
| `gpuhash_xxhash64_prepared` | 3.0567 ms | 163.57 GiB/s | 6.68x |
| `twox_hash_xxhash3_64_cpu` | 10.934 ms | 45.729 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_prepared` | 3.2281 ms | 154.89 GiB/s | 3.39x |
| `twox_hash_xxhash3_128_cpu` | 11.316 ms | 44.185 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_prepared` | 3.2633 ms | 153.22 GiB/s | 3.47x |
| `twox_hash_xxhash3_64_secret_cpu` | 11.411 ms | 43.818 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_secret_prepared` | 3.2538 ms | 153.67 GiB/s | 3.51x |
| `twox_hash_xxhash3_128_secret_cpu` | 11.563 ms | 43.240 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_secret_prepared` | 3.2883 ms | 152.05 GiB/s | 3.52x |
| `rustcrypto_sha256_cpu` | 165.50 ms | 3.0211 GiB/s | 1.0x |
| `gpuhash_sha256_prepared` | 12.813 ms | 39.024 GiB/s | 12.92x |

The quick benchmark is useful for a one-shot sanity check:

```sh
cargo run --release --example quick_bench
```

It also accepts optional `count len` arguments:

```sh
cargo run --release --example quick_bench -- 131072 4096
```
