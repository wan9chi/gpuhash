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

## Performance

On an Apple M5 Pro with a prepared 512 MiB batch, Criterion reports:

| Algorithm | CPU baseline | GPU prepared | Speedup |
| --- | ---: | ---: | ---: |
| `twox-hash` XXH32 | 66.105 ms | 3.0513 ms | 21.66x |
| `twox-hash` XXH64 | 20.363 ms | 3.0380 ms | 6.70x |
| `twox-hash` XXH3-64 | 10.869 ms | 3.0494 ms | 3.56x |
| `twox-hash` XXH3-128 | 11.314 ms | 3.0709 ms | 3.68x |
| `twox-hash` XXH3-64 custom secret | 11.310 ms | 3.0514 ms | 3.71x |
| `twox-hash` XXH3-128 custom secret | 11.468 ms | 3.0983 ms | 3.70x |
| RustCrypto SHA-256 | 165.14 ms | 9.3247 ms | 17.71x |

See [BENCHMARKS.md](BENCHMARKS.md) for the full command and throughput table.
The Apple Silicon benchmark job runs in
[GitHub Actions](https://github.com/wan9chi/gpuhash/actions/workflows/ci.yml?query=branch%3Amain);
each run uploads the Criterion report as a `criterion-report` artifact.
Example successful benchmark job:
[run 27379358286 / job 80911929636](https://github.com/wan9chi/gpuhash/actions/runs/27379358286/job/80911929636),
with `criterion-report` artifact id `7577691995`.

## Run

```sh
cargo test
cargo run --release --example quick_bench
cargo bench --bench hash_batch
```

GPU acceleration is a batch API. Single tiny messages are still better served by
the CPU reference crates because Metal command submission has fixed overhead.
The fastest path is `PreparedBatch` reuse; buffered GPU-backed hasher helpers are
also available for `std::hash::Hasher`- and `BuildHasher`-style compatibility
when ergonomics matter more than peak throughput.
