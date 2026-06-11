# gpuhash

[![CI](https://github.com/wan9chi/gpuhash/actions/workflows/ci.yml/badge.svg)](https://github.com/wan9chi/gpuhash/actions/workflows/ci.yml)

Apple Silicon GPU batch hashing for:

- `xxHash32`, compatible with `twox-hash::XxHash32::oneshot`
- `xxHash64`, compatible with `twox-hash::XxHash64::oneshot`
- `XXH3-64`, compatible with `twox-hash::XxHash3_64::oneshot` and `oneshot_with_seed`
- `XXH3-128`, compatible with `twox-hash::XxHash3_128::oneshot` and `oneshot_with_seed`
- SHA-256, compatible with RustCrypto `sha2::Sha256`

The crate uses Metal compute kernels and `StorageModeShared` buffers. On Apple
Silicon that means CPU and GPU access the same unified-memory allocation; the
`PreparedBatch` API lets callers upload/pack messages once and reuse the shared
buffers across many hash launches.

## Performance

On an Apple M5 Pro with a prepared 512 MiB batch, Criterion reports:

| Algorithm | CPU baseline | GPU prepared | Speedup |
| --- | ---: | ---: | ---: |
| `twox-hash` XXH32 | 65.670 ms | 3.1027 ms | 21.16x |
| `twox-hash` XXH64 | 20.417 ms | 3.1626 ms | 6.46x |
| `twox-hash` XXH3-64 | 10.910 ms | 3.3496 ms | 3.26x |
| `twox-hash` XXH3-128 | 11.396 ms | 3.5065 ms | 3.25x |
| RustCrypto SHA-256 | 165.90 ms | 13.011 ms | 12.75x |

See [BENCHMARKS.md](BENCHMARKS.md) for the full command and throughput table.
The Apple Silicon benchmark job runs in
[GitHub Actions](https://github.com/wan9chi/gpuhash/actions/workflows/ci.yml?query=branch%3Amain);
the latest verified run is
[CI #27371941256](https://github.com/wan9chi/gpuhash/actions/runs/27371941256),
with the
[Benchmark on Apple Silicon job](https://github.com/wan9chi/gpuhash/actions/runs/27371941256/job/80886383482)
and
[criterion-report artifact](https://github.com/wan9chi/gpuhash/actions/runs/27371941256/artifacts/7574779190).

## Run

```sh
cargo test
cargo run --release --example quick_bench
cargo bench --bench hash_batch
```

GPU acceleration is a batch API. Single tiny messages are still better served by
the CPU reference crates because Metal command submission has fixed overhead.
This crate targets one-shot batch hashing; it does not implement the streaming
`Hasher` traits or custom-secret XXH3 APIs from `twox-hash`.
