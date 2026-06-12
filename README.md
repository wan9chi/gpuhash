# gpuhash

[![CI](https://github.com/wan9chi/gpuhash/actions/workflows/ci.yml/badge.svg)](https://github.com/wan9chi/gpuhash/actions/workflows/ci.yml)

Apple Silicon GPU batch hashing for:

- `xxHash32`, compatible with `twox-hash::XxHash32::oneshot`
- `xxHash64`, compatible with `twox-hash::XxHash64::oneshot`
- `XXH3-64`, compatible with `twox-hash::XxHash3_64::oneshot`,
  `oneshot_with_seed`, `oneshot_with_secret`, and
  `oneshot_with_seed_and_secret`
- `XXH3-128`, compatible with `twox-hash::XxHash3_128::oneshot`,
  `oneshot_with_seed`, `oneshot_with_secret`, and
  `oneshot_with_seed_and_secret`
- SHA-256, compatible with RustCrypto `sha2::Sha256` and the
  `digest::Digest` trait surface

The crate uses Metal compute kernels and `StorageModeShared` buffers. On Apple
Silicon that means CPU and GPU access the same unified-memory allocation; the
`PreparedBatch` API lets callers upload/pack messages once and reuse the shared
buffers across many hash launches. Result vectors are wrapped as no-copy shared
Metal buffers, so kernels write directly into the Rust output allocation.
Small kernel configuration structs are bound inline with Metal `setBytes`,
avoiding per-dispatch config-buffer allocation. XXH3 secrets up to 4 KiB use
the same inline path; larger custom secrets fall back to shared Metal buffers.
For file-heavy callers, `PreparedBatchBuilder` can reuse the shared input and
descriptor buffers and fill message slots directly, including a parallel fill
mode suitable for reading many independent files straight into unified memory.

## Performance

On an Apple M5 Pro with a prepared 512 MiB batch, Criterion reports:

| Algorithm | CPU baseline | GPU prepared | Speedup |
| --- | ---: | ---: | ---: |
| `twox-hash` XXH32 | 68.136 ms | 3.0371 ms | 22.43x |
| `twox-hash` XXH64 | 20.982 ms | 2.9981 ms | 7.00x |
| `twox-hash` XXH3-64 | 11.748 ms | 3.0316 ms | 3.88x |
| `twox-hash` XXH3-128 | 12.340 ms | 3.0239 ms | 4.08x |
| `twox-hash` XXH3-64 custom secret | 12.026 ms | 3.0143 ms | 3.99x |
| `twox-hash` XXH3-128 custom secret | 13.170 ms | 3.0764 ms | 4.28x |
| RustCrypto SHA-256 | 165.69 ms | 9.1328 ms | 18.14x |

See [BENCHMARKS.md](BENCHMARKS.md) for the full command and throughput table.
The Apple Silicon benchmark job runs in
[GitHub Actions](https://github.com/wan9chi/gpuhash/actions/workflows/ci.yml?query=branch%3Amain);
each run uploads the Criterion report as a `criterion-report` artifact.
Example successful benchmark job:
[run 27389494910 / job 80943862081](https://github.com/wan9chi/gpuhash/actions/runs/27389494910/job/80943862081),
with [`criterion-report` artifact id `7581404441`](https://github.com/wan9chi/gpuhash/actions/runs/27389494910/artifacts/7581404441).

For a real frontend tree, the CI small-files benchmark clones Vite, installs
`node_modules`, and hashes every regular file, including real package files
under `node_modules/.pnpm`. On the Apple M1 virtual GitHub runner, the
1.5 MiB hybrid threshold sends 39,598 files / 267.82 MiB through direct GPU
buffer fill and 36 larger files / 381.45 MiB through CPU hashing:

| Workload | CPU read + hash | Hybrid direct GPU/CPU | Speedup |
| --- | ---: | ---: | ---: |
| Vite tree XXH3-64 | 5475.702 ms | 3027.313 ms | 1.809x |
| Vite tree SHA-256 | 5735.252 ms | 3174.173 ms | 1.807x |

Small-files benchmark job:
[run 27389494910 / job 80943862098](https://github.com/wan9chi/gpuhash/actions/runs/27389494910/job/80943862098),
with [`small-files-benchmark` artifact id `7581383990`](https://github.com/wan9chi/gpuhash/actions/runs/27389494910/artifacts/7581383990).

## Run

```sh
cargo test
cargo run --release --example quick_bench
cargo bench --bench hash_batch
cargo bench --bench small_files -- --root target/small-files/vite
```

GPU acceleration is a batch API. Single tiny messages are still better served by
the CPU reference crates because Metal command submission has fixed overhead.
The fastest path is `PreparedBatch` reuse; buffered GPU-backed hasher helpers are
also available for `std::hash::Hasher`- and `BuildHasher`-style compatibility
when ergonomics matter more than peak throughput.
For file trees with large outliers, the small-files benchmark reports both
all-GPU direct-read and hybrid GPU/CPU totals because SHA-256 and large-message
hash chains are serial enough that CPU fallback can be the better real-world
choice.
