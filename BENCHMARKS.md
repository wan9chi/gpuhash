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
[run 27380736769 / job 80916683715](https://github.com/wan9chi/gpuhash/actions/runs/27380736769/job/80916683715),
with `criterion-report` artifact id `7578233922`.

Benchmark batch:

- 131,072 messages
- 4,096 bytes per message
- 512 MiB total input
- custom-secret XXH3 rows use a 257-byte secret
- GPU data path uses `PreparedBatch` shared Metal buffers
- uniform-length prepared batches bypass descriptor-buffer loads and compute
  offsets from `gid * stride`
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
| `twox_hash_xxhash32_cpu` | 67.502 ms | 7.4071 GiB/s | 1.0x |
| `gpuhash_xxhash32_prepared` | 3.0519 ms | 163.83 GiB/s | 22.12x |
| `twox_hash_xxhash64_cpu` | 20.498 ms | 24.392 GiB/s | 1.0x |
| `gpuhash_xxhash64_prepared` | 3.0344 ms | 164.78 GiB/s | 6.76x |
| `twox_hash_xxhash3_64_cpu` | 11.127 ms | 44.934 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_prepared` | 3.0376 ms | 164.60 GiB/s | 3.66x |
| `twox_hash_xxhash3_128_cpu` | 11.979 ms | 41.739 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_prepared` | 3.0241 ms | 165.34 GiB/s | 3.96x |
| `twox_hash_xxhash3_64_secret_cpu` | 11.563 ms | 43.240 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_secret_prepared` | 3.0220 ms | 165.45 GiB/s | 3.83x |
| `twox_hash_xxhash3_128_secret_cpu` | 12.069 ms | 41.428 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_secret_prepared` | 3.0557 ms | 163.63 GiB/s | 3.95x |
| `rustcrypto_sha256_cpu` | 165.99 ms | 3.0122 GiB/s | 1.0x |
| `gpuhash_sha256_prepared` | 9.1334 ms | 54.744 GiB/s | 18.17x |

The quick benchmark is useful for a one-shot sanity check:

```sh
cargo run --release --example quick_bench
```

It also accepts optional `count len` arguments:

```sh
cargo run --release --example quick_bench -- 131072 4096
```
