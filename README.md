# gpuhash

[![CI](https://github.com/wan9chi/gpuhash/actions/workflows/ci.yml/badge.svg)](https://github.com/wan9chi/gpuhash/actions/workflows/ci.yml)

Apple Silicon GPU batch hashing for:

- `xxHash32`, compatible with `twox-hash::XxHash32::oneshot`
- `xxHash64`, compatible with `twox-hash::XxHash64::oneshot`
- `XXH3-64`, compatible with `twox-hash::XxHash3_64::oneshot`,
  `oneshot_with_seed`, and `oneshot_with_secret`
- `XXH3-128`, compatible with `twox-hash::XxHash3_128::oneshot`,
  `oneshot_with_seed`, and `oneshot_with_secret`
- SHA-256, compatible with RustCrypto `sha2::Sha256`

The crate uses Metal compute kernels and `StorageModeShared` buffers. On Apple
Silicon that means CPU and GPU access the same unified-memory allocation; the
`PreparedBatch` API lets callers upload/pack messages once and reuse the shared
buffers across many hash launches. Result vectors are wrapped as no-copy shared
Metal buffers, so kernels write directly into the Rust output allocation.

## Performance

On an Apple M5 Pro with a prepared 512 MiB batch, Criterion reports:

| Algorithm | CPU baseline | GPU prepared | Speedup |
| --- | ---: | ---: | ---: |
| `twox-hash` XXH32 | 66.061 ms | 3.0584 ms | 21.60x |
| `twox-hash` XXH64 | 20.409 ms | 3.0567 ms | 6.68x |
| `twox-hash` XXH3-64 | 10.934 ms | 3.2281 ms | 3.39x |
| `twox-hash` XXH3-128 | 11.316 ms | 3.2633 ms | 3.47x |
| `twox-hash` XXH3-64 custom secret | 11.411 ms | 3.2538 ms | 3.51x |
| `twox-hash` XXH3-128 custom secret | 11.563 ms | 3.2883 ms | 3.52x |
| RustCrypto SHA-256 | 165.50 ms | 12.813 ms | 12.92x |

See [BENCHMARKS.md](BENCHMARKS.md) for the full command and throughput table.
The Apple Silicon benchmark job runs in
[GitHub Actions](https://github.com/wan9chi/gpuhash/actions/workflows/ci.yml?query=branch%3Amain);
each run uploads the Criterion report as a `criterion-report` artifact.

## Run

```sh
cargo test
cargo run --release --example quick_bench
cargo bench --bench hash_batch
```

GPU acceleration is a batch API. Single tiny messages are still better served by
the CPU reference crates because Metal command submission has fixed overhead.
This crate targets one-shot batch hashing; it does not implement the streaming
`Hasher` traits from `twox-hash`.
