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
[run 27389494910 / job 80943862081](https://github.com/wan9chi/gpuhash/actions/runs/27389494910/job/80943862081),
with [`criterion-report` artifact id `7581404441`](https://github.com/wan9chi/gpuhash/actions/runs/27389494910/artifacts/7581404441).

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
- small kernel configuration structs are bound inline with Metal `setBytes`
  instead of allocating per-dispatch config buffers
- XXH3 secrets up to 4 KiB are also bound inline, with larger custom secrets
  falling back to shared Metal buffers
- XXH3 large-message accumulation uses aligned 64-bit loads for regular
  message and secret stripes
- SHA-256 compresses full 64-byte data blocks directly and only uses padded
  reads for the final block(s)

Criterion results:

| Benchmark | Mean time | Throughput | Speedup |
| --- | ---: | ---: | ---: |
| `twox_hash_xxhash32_cpu` | 68.136 ms | 7.3382 GiB/s | 1.0x |
| `gpuhash_xxhash32_prepared` | 3.0371 ms | 164.63 GiB/s | 22.43x |
| `twox_hash_xxhash64_cpu` | 20.982 ms | 23.830 GiB/s | 1.0x |
| `gpuhash_xxhash64_prepared` | 2.9981 ms | 166.77 GiB/s | 7.00x |
| `twox_hash_xxhash3_64_cpu` | 11.748 ms | 42.559 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_prepared` | 3.0316 ms | 164.93 GiB/s | 3.88x |
| `twox_hash_xxhash3_128_cpu` | 12.340 ms | 40.517 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_prepared` | 3.0239 ms | 165.35 GiB/s | 4.08x |
| `twox_hash_xxhash3_64_secret_cpu` | 12.026 ms | 41.575 GiB/s | 1.0x |
| `gpuhash_xxhash3_64_secret_prepared` | 3.0143 ms | 165.88 GiB/s | 3.99x |
| `twox_hash_xxhash3_128_secret_cpu` | 13.170 ms | 37.964 GiB/s | 1.0x |
| `gpuhash_xxhash3_128_secret_prepared` | 3.0764 ms | 162.53 GiB/s | 4.28x |
| `rustcrypto_sha256_cpu` | 165.69 ms | 3.0177 GiB/s | 1.0x |
| `gpuhash_sha256_prepared` | 9.1328 ms | 54.748 GiB/s | 18.14x |

The quick benchmark is useful for a one-shot sanity check:

```sh
cargo run --release --example quick_bench
```

It also accepts optional `count len` arguments:

```sh
cargo run --release --example quick_bench -- 131072 4096
```

## Small Files Benchmark

The small-files benchmark measures a frontend project tree rather than a
synthetic fixed-size batch. The CI job clones
[`vitejs/vite`](https://github.com/vitejs/vite) at
`d64a1a5557b3caea9469e70b647ff2c9d9def809`, installs `node_modules` with the
repo-pinned `pnpm`, then hashes every regular file under the checkout. It skips
`.git` and symlinks so pnpm package links are not double-counted; real files
inside `node_modules/.pnpm` are included.

Fixture setup:

```sh
bash scripts/setup_small_files_fixture.sh target/small-files/vite
```

Benchmark command:

```sh
cargo bench --locked --bench small_files -- --root target/small-files/vite
```

The benchmark verifies every GPU and hybrid output against CPU output. It
reports:

- CPU baseline: read every file into per-file buffers, then hash with the
  reference crates.
- Copy-prepared GPU path: read into per-file buffers, copy into a prepared
  shared Metal batch, then hash on GPU.
- All-files direct GPU path: read every file directly into a reusable shared
  Metal buffer with `PreparedBatchBuilder::prepare_with_lengths_parallel`.
- Hybrid path: read files at or below the threshold directly into the reusable
  Metal buffer and hash them on GPU; read larger files into CPU buffers and hash
  them on CPU. The default threshold is 1.5 MiB and can be changed with
  `--gpu-max-file-bytes` or `GPUHASH_SMALL_FILES_GPU_MAX_BYTES`.

Example CI small-files job:
[run 27389494910 / job 80943862098](https://github.com/wan9chi/gpuhash/actions/runs/27389494910/job/80943862098),
with [`small-files-benchmark` artifact id `7581383990`](https://github.com/wan9chi/gpuhash/actions/runs/27389494910/artifacts/7581383990).

CI fixture from that run:

- Apple M1 (Virtual), macOS 15.7.7, Rust 1.95.0
- 39,634 files
- 680,803,695 bytes (649.26 MiB)
- Hybrid threshold: 1,572,864 bytes
- Hybrid GPU partition: 39,598 files, 280,826,187 bytes (267.82 MiB)
- Hybrid CPU partition: 36 files, 399,977,508 bytes (381.45 MiB)

CI small-files results:

| Path | XXH3-64 total | XXH3 speedup | SHA-256 total | SHA-256 speedup |
| --- | ---: | ---: | ---: | ---: |
| CPU read + hash | 5475.702 ms | 1.000x | 5735.252 ms | 1.000x |
| GPU copy-prepared | 8697.111 ms | 0.630x | 16058.405 ms | 0.357x |
| GPU direct all files | 2915.929 ms | 1.878x | 10131.072 ms | 0.566x |
| Hybrid direct GPU/CPU | 3027.313 ms | 1.809x | 3174.173 ms | 1.807x |

The all-GPU direct path is excellent for this CI runner's XXH3 file traversal,
but SHA-256 remains serial enough on large files that the hybrid path is the
better combined real-world strategy.
