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
A verified benchmark run is
[CI #27371941256](https://github.com/wan9chi/gpuhash/actions/runs/27371941256);
its benchmark output is in the
[Benchmark on Apple Silicon job](https://github.com/wan9chi/gpuhash/actions/runs/27371941256/job/80886383482)
and the
[criterion-report artifact](https://github.com/wan9chi/gpuhash/actions/runs/27371941256/artifacts/7574779190).

Benchmark batch:

- 131,072 messages
- 4,096 bytes per message
- 512 MiB total input
- custom-secret XXH3 rows use a 257-byte secret
- GPU data path uses `PreparedBatch` shared Metal buffers

Criterion results:

| Benchmark | Mean time | Throughput | Speedup |
| --- | ---: | ---: | ---: |
| `twox_hash_xxhash32_cpu` | 65.833 ms | 7.5950 GiB/s | 1.0x |
| `gpuhash_xxhash32_prepared` | 3.1181 ms | 160.36 GiB/s | 21.11x |
| `twox_hash_xxhash64_cpu` | 20.379 ms | 24.535 GiB/s | 1.0x |
| `gpuhash_xxhash64_prepared` | 3.1705 ms | 157.70 GiB/s | 6.43x |
| `twox_hash_xxhash3_64_cpu` | 10.924 ms | 45.769 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_prepared` | 3.3516 ms | 149.18 GiB/s | 3.26x |
| `twox_hash_xxhash3_128_cpu` | 11.552 ms | 43.284 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_prepared` | 3.5254 ms | 141.83 GiB/s | 3.28x |
| `twox_hash_xxhash3_64_secret_cpu` | 11.397 ms | 43.872 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_secret_prepared` | 3.3484 ms | 149.33 GiB/s | 3.40x |
| `twox_hash_xxhash3_128_secret_cpu` | 11.530 ms | 43.364 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_secret_prepared` | 3.5698 ms | 140.07 GiB/s | 3.23x |
| `rustcrypto_sha256_cpu` | 165.65 ms | 3.0185 GiB/s | 1.0x |
| `gpuhash_sha256_prepared` | 13.002 ms | 38.455 GiB/s | 12.74x |

The quick benchmark is useful for a one-shot sanity check:

```sh
cargo run --release --example quick_bench
```

It also accepts optional `count len` arguments:

```sh
cargo run --release --example quick_bench -- 131072 4096
```
