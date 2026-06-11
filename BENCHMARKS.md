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
- GPU data path uses `PreparedBatch` shared Metal buffers

Criterion results:

| Benchmark | Mean time | Throughput | Speedup |
| --- | ---: | ---: | ---: |
| `twox_hash_xxhash32_cpu` | 65.670 ms | 7.6139 GiB/s | 1.0x |
| `gpuhash_xxhash32_prepared` | 3.1027 ms | 161.15 GiB/s | 21.16x |
| `twox_hash_xxhash64_cpu` | 20.417 ms | 24.490 GiB/s | 1.0x |
| `gpuhash_xxhash64_prepared` | 3.1626 ms | 158.10 GiB/s | 6.46x |
| `twox_hash_xxhash3_64_cpu` | 10.910 ms | 45.830 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_prepared` | 3.3496 ms | 149.27 GiB/s | 3.26x |
| `twox_hash_xxhash3_128_cpu` | 11.396 ms | 43.876 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_prepared` | 3.5065 ms | 142.59 GiB/s | 3.25x |
| `rustcrypto_sha256_cpu` | 165.90 ms | 3.0138 GiB/s | 1.0x |
| `gpuhash_sha256_prepared` | 13.011 ms | 38.430 GiB/s | 12.75x |

The quick benchmark is useful for a one-shot sanity check:

```sh
cargo run --release --example quick_bench
```

It also accepts optional `count len` arguments:

```sh
cargo run --release --example quick_bench -- 131072 4096
```
