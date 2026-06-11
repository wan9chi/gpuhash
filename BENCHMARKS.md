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
Example successful benchmark job:
[run 27379358286 / job 80911929636](https://github.com/wan9chi/gpuhash/actions/runs/27379358286/job/80911929636),
with `criterion-report` artifact id `7577691995`.

Benchmark batch:

- 131,072 messages
- 4,096 bytes per message
- 512 MiB total input
- custom-secret XXH3 rows use a 257-byte secret
- GPU data path uses `PreparedBatch` shared Metal buffers
- GPU output buffers wrap the returned Rust `Vec` allocation directly with
  `newBufferWithBytesNoCopy`
- GPU result vectors skip CPU zero-fill and mark outputs initialized only after
  the command buffer completes
- XXH3 large-message accumulation uses aligned 64-bit loads for regular
  message and secret stripes
- SHA-256 compresses full 64-byte data blocks directly and only uses padded
  reads for the final block(s)

Criterion results:

| Benchmark | Mean time | Throughput | Speedup |
| --- | ---: | ---: | ---: |
| `twox_hash_xxhash32_cpu` | 66.105 ms | 7.5638 GiB/s | 1.0x |
| `gpuhash_xxhash32_prepared` | 3.0513 ms | 163.87 GiB/s | 21.66x |
| `twox_hash_xxhash64_cpu` | 20.363 ms | 24.554 GiB/s | 1.0x |
| `gpuhash_xxhash64_prepared` | 3.0380 ms | 164.58 GiB/s | 6.70x |
| `twox_hash_xxhash3_64_cpu` | 10.869 ms | 46.002 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_prepared` | 3.0494 ms | 163.97 GiB/s | 3.56x |
| `twox_hash_xxhash3_128_cpu` | 11.314 ms | 44.193 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_prepared` | 3.0709 ms | 162.82 GiB/s | 3.68x |
| `twox_hash_xxhash3_64_secret_cpu` | 11.310 ms | 44.207 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_secret_prepared` | 3.0514 ms | 163.86 GiB/s | 3.71x |
| `twox_hash_xxhash3_128_secret_cpu` | 11.468 ms | 43.600 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_secret_prepared` | 3.0983 ms | 161.38 GiB/s | 3.70x |
| `rustcrypto_sha256_cpu` | 165.14 ms | 3.0277 GiB/s | 1.0x |
| `gpuhash_sha256_prepared` | 9.3247 ms | 53.621 GiB/s | 17.71x |

The quick benchmark is useful for a one-shot sanity check:

```sh
cargo run --release --example quick_bench
```

It also accepts optional `count len` arguments:

```sh
cargo run --release --example quick_bench -- 131072 4096
```
