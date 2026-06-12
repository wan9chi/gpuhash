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

The benchmark reads all files once into per-file buffers, verifies GPU output
against CPU output, and reports both hash-only phases and end-to-end totals
using the common read time.
