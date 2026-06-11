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
- SHA-256 compresses full 64-byte data blocks directly and only uses padded
  reads for the final block(s)

Criterion results:

| Benchmark | Mean time | Throughput | Speedup |
| --- | ---: | ---: | ---: |
| `twox_hash_xxhash32_cpu` | 65.790 ms | 7.5999 GiB/s | 1.0x |
| `gpuhash_xxhash32_prepared` | 3.0440 ms | 164.26 GiB/s | 21.61x |
| `twox_hash_xxhash64_cpu` | 20.512 ms | 24.376 GiB/s | 1.0x |
| `gpuhash_xxhash64_prepared` | 3.0477 ms | 164.06 GiB/s | 6.73x |
| `twox_hash_xxhash3_64_cpu` | 10.914 ms | 45.812 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_prepared` | 3.2268 ms | 154.95 GiB/s | 3.38x |
| `twox_hash_xxhash3_128_cpu` | 11.441 ms | 43.704 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_prepared` | 3.2527 ms | 153.72 GiB/s | 3.52x |
| `twox_hash_xxhash3_64_secret_cpu` | 11.378 ms | 43.944 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_secret_prepared` | 3.2343 ms | 154.59 GiB/s | 3.52x |
| `twox_hash_xxhash3_128_secret_cpu` | 11.739 ms | 42.594 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_secret_prepared` | 3.2502 ms | 153.84 GiB/s | 3.61x |
| `rustcrypto_sha256_cpu` | 165.10 ms | 3.0285 GiB/s | 1.0x |
| `gpuhash_sha256_prepared` | 9.3139 ms | 53.683 GiB/s | 17.73x |

The quick benchmark is useful for a one-shot sanity check:

```sh
cargo run --release --example quick_bench
```

It also accepts optional `count len` arguments:

```sh
cargo run --release --example quick_bench -- 131072 4096
```
