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
[run 27378146796 / job 80907945549](https://github.com/wan9chi/gpuhash/actions/runs/27378146796/job/80907945549),
with `criterion-report` artifact id `7577230947`.

Benchmark batch:

- 131,072 messages
- 4,096 bytes per message
- 512 MiB total input
- custom-secret XXH3 rows use a 257-byte secret
- GPU data path uses `PreparedBatch` shared Metal buffers
- GPU output buffers wrap the returned Rust `Vec` allocation directly with
  `newBufferWithBytesNoCopy`
- XXH3 large-message accumulation uses aligned 64-bit loads for regular
  message and secret stripes
- SHA-256 compresses full 64-byte data blocks directly and only uses padded
  reads for the final block(s)

Criterion results:

| Benchmark | Mean time | Throughput | Speedup |
| --- | ---: | ---: | ---: |
| `twox_hash_xxhash32_cpu` | 65.244 ms | 7.6636 GiB/s | 1.0x |
| `gpuhash_xxhash32_prepared` | 3.1044 ms | 161.06 GiB/s | 21.02x |
| `twox_hash_xxhash64_cpu` | 20.310 ms | 24.618 GiB/s | 1.0x |
| `gpuhash_xxhash64_prepared` | 3.0536 ms | 163.74 GiB/s | 6.65x |
| `twox_hash_xxhash3_64_cpu` | 10.879 ms | 45.961 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_prepared` | 3.0647 ms | 163.15 GiB/s | 3.55x |
| `twox_hash_xxhash3_128_cpu` | 11.297 ms | 44.259 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_prepared` | 3.0942 ms | 161.59 GiB/s | 3.65x |
| `twox_hash_xxhash3_64_secret_cpu` | 11.318 ms | 44.176 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_secret_prepared` | 3.0731 ms | 162.70 GiB/s | 3.68x |
| `twox_hash_xxhash3_128_secret_cpu` | 11.403 ms | 43.847 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_secret_prepared` | 3.1257 ms | 159.96 GiB/s | 3.65x |
| `rustcrypto_sha256_cpu` | 165.36 ms | 3.0236 GiB/s | 1.0x |
| `gpuhash_sha256_prepared` | 9.4080 ms | 53.146 GiB/s | 17.58x |

The quick benchmark is useful for a one-shot sanity check:

```sh
cargo run --release --example quick_bench
```

It also accepts optional `count len` arguments:

```sh
cargo run --release --example quick_bench -- 131072 4096
```
