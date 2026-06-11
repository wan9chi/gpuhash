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
| `twox-hash` XXH32 | 67.650 ms | 3.0395 ms | 22.26x |
| `twox-hash` XXH64 | 20.650 ms | 3.0652 ms | 6.74x |
| `twox-hash` XXH3-64 | 11.197 ms | 3.2339 ms | 3.46x |
| `twox-hash` XXH3-128 | 11.687 ms | 3.2698 ms | 3.57x |
| `twox-hash` XXH3-64 custom secret | 11.500 ms | 3.2626 ms | 3.52x |
| `twox-hash` XXH3-128 custom secret | 12.271 ms | 3.2812 ms | 3.74x |
| RustCrypto SHA-256 | 165.19 ms | 9.7215 ms | 16.99x |

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
The fastest path is `PreparedBatch` reuse; buffered GPU-backed hasher helpers are
also available for `std::hash::Hasher`-style compatibility when ergonomics matter
more than peak throughput.
