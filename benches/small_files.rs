use gpuhash::{GpuHash, PreparedBatchBuilder};
use sha2::{Digest as _, Sha256};
use std::{
    env, fs,
    fs::File,
    io,
    io::Read as _,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};
use twox_hash::XxHash3_64;

const DEFAULT_HYBRID_GPU_MAX_FILE_BYTES: u64 = 1536 * 1024;

#[derive(Debug)]
struct Args {
    root: PathBuf,
    repetitions: usize,
    gpu_max_file_bytes: u64,
}

#[derive(Debug, Clone)]
struct FileEntry {
    path: PathBuf,
    len: u64,
}

#[derive(Debug)]
struct FileSet {
    files: Vec<FileEntry>,
    total_bytes: u64,
    skipped_dirs: usize,
    skipped_symlinks: usize,
    scan_elapsed: Duration,
}

#[derive(Debug)]
struct CpuRun<T> {
    output: Vec<T>,
    elapsed: Duration,
}

#[derive(Debug)]
struct GpuHashRun<T> {
    output: Vec<T>,
    hash: Duration,
}

#[derive(Debug)]
struct HybridPartition {
    gpu_files: Vec<FileEntry>,
    gpu_indices: Vec<usize>,
    gpu_bytes: u64,
    cpu_files: Vec<FileEntry>,
    cpu_indices: Vec<usize>,
    cpu_bytes: u64,
}

#[derive(Debug)]
struct Timed<T> {
    output: T,
    elapsed: Duration,
}

struct HybridInput {
    small_batch: gpuhash::PreparedBatch,
    large_messages: Vec<Vec<u8>>,
    small_prepare: Duration,
    large_read: Duration,
    wall: Duration,
}

#[derive(Debug)]
struct HybridHashRun<T> {
    small_output: Vec<T>,
    large_output: Vec<T>,
    small_hash: Duration,
    large_hash: Duration,
    wall: Duration,
}

fn main() -> gpuhash::Result<()> {
    let args = parse_args()?;
    let root = fs::canonicalize(&args.root).map_err(|err| {
        gpuhash::GpuHashError::Metal(format!(
            "failed to canonicalize {}: {err}",
            args.root.display()
        ))
    })?;

    println!("small-files fixture: {}", root.display());
    println!("note: regular files are included; .git and symlinks are skipped");

    let file_set = scan_files(&root)?;
    println!(
        "files: {}, bytes: {} ({:.2} MiB), scan: {:.3} ms, skipped_dirs: {}, skipped_symlinks: {}",
        file_set.files.len(),
        file_set.total_bytes,
        file_set.total_bytes as f64 / (1024.0 * 1024.0),
        ms(file_set.scan_elapsed),
        file_set.skipped_dirs,
        file_set.skipped_symlinks
    );

    if file_set.files.is_empty() {
        return Err(gpuhash::GpuHashError::Metal(format!(
            "no regular files found under {}",
            root.display()
        )));
    }

    let hybrid = partition_for_hybrid(&file_set.files, args.gpu_max_file_bytes);
    println!(
        "hybrid threshold: files <= {} bytes use GPU direct-read; larger files use CPU",
        args.gpu_max_file_bytes
    );
    println!(
        "hybrid gpu files: {}, bytes: {} ({:.2} MiB); cpu files: {}, bytes: {} ({:.2} MiB)",
        hybrid.gpu_files.len(),
        hybrid.gpu_bytes,
        hybrid.gpu_bytes as f64 / (1024.0 * 1024.0),
        hybrid.cpu_files.len(),
        hybrid.cpu_bytes,
        hybrid.cpu_bytes as f64 / (1024.0 * 1024.0),
    );

    let gpu_start = Instant::now();
    let gpu = GpuHash::new()?;
    println!("metal init: {:.3} ms", ms(gpu_start.elapsed()));
    let mut direct_builder = gpu.prepared_batch_builder();
    let mut hybrid_builder = gpu.prepared_batch_builder();

    for rep in 0..args.repetitions {
        println!();
        println!("repetition: {}/{}", rep + 1, args.repetitions);

        let hybrid_input = prepare_hybrid_input(&mut hybrid_builder, &hybrid)?;
        print_phase(
            "hybrid_small_direct_read",
            hybrid_input.small_prepare,
            hybrid.gpu_bytes,
        );
        print_phase(
            "hybrid_large_read_files",
            hybrid_input.large_read,
            hybrid.cpu_bytes,
        );
        print_phase("hybrid_input_wall", hybrid_input.wall, file_set.total_bytes);

        let hybrid_xxh3 = hybrid_xxh3(
            &gpu,
            &hybrid_input.small_batch,
            &hybrid_input.large_messages,
        )?;
        print_phase(
            "hybrid_gpu_xxh3_small_hash",
            hybrid_xxh3.small_hash,
            hybrid.gpu_bytes,
        );
        print_phase(
            "hybrid_cpu_xxh3_large_hash",
            hybrid_xxh3.large_hash,
            hybrid.cpu_bytes,
        );
        print_phase(
            "hybrid_xxh3_hash_wall",
            hybrid_xxh3.wall,
            file_set.total_bytes,
        );

        let hybrid_sha256 = hybrid_sha256(
            &gpu,
            &hybrid_input.small_batch,
            &hybrid_input.large_messages,
        )?;
        print_phase(
            "hybrid_gpu_sha256_small_hash",
            hybrid_sha256.small_hash,
            hybrid.gpu_bytes,
        );
        print_phase(
            "hybrid_cpu_sha256_large_hash",
            hybrid_sha256.large_hash,
            hybrid.cpu_bytes,
        );
        print_phase(
            "hybrid_sha256_hash_wall",
            hybrid_sha256.wall,
            file_set.total_bytes,
        );

        let direct_prepare_start = Instant::now();
        let direct_batch = prepare_files_direct(&mut direct_builder, &file_set.files)?;
        let direct_prepare = direct_prepare_start.elapsed();
        print_phase(
            "gpu_prepare_batch_direct_all",
            direct_prepare,
            file_set.total_bytes,
        );

        let gpu_xxh3_direct = gpu_xxh3(&gpu, &direct_batch)?;
        let gpu_sha256_direct = gpu_sha256(&gpu, &direct_batch)?;

        let read_start = Instant::now();
        let messages = read_all(&file_set.files)?;
        let read = read_start.elapsed();
        print_phase("read_all_files", read, file_set.total_bytes);

        let prepare_start = Instant::now();
        let batch = gpu.prepare_batch(&messages)?;
        let prepare = prepare_start.elapsed();
        print_phase("gpu_prepare_batch_copy", prepare, file_set.total_bytes);

        let cpu_xxh3 = cpu_xxh3(&messages);
        let gpu_xxh3_copy = gpu_xxh3(&gpu, &batch)?;
        if cpu_xxh3.output != gpu_xxh3_copy.output || cpu_xxh3.output != gpu_xxh3_direct.output {
            return Err(gpuhash::GpuHashError::Metal(
                "XXH3 CPU/GPU output mismatch".to_owned(),
            ));
        }
        verify_partitioned(
            &cpu_xxh3.output,
            &hybrid.gpu_indices,
            &hybrid_xxh3.small_output,
            &hybrid.cpu_indices,
            &hybrid_xxh3.large_output,
            "XXH3 hybrid output mismatch",
        )?;
        print_phase(
            "cpu_xxh3_hash_buffers",
            cpu_xxh3.elapsed,
            file_set.total_bytes,
        );
        print_phase(
            "gpu_xxh3_hash_copy_prepared",
            gpu_xxh3_copy.hash,
            file_set.total_bytes,
        );
        print_phase(
            "gpu_xxh3_hash_direct_all_prepared",
            gpu_xxh3_direct.hash,
            file_set.total_bytes,
        );
        print_total(
            "cpu_xxh3_total_read_hash",
            read + cpu_xxh3.elapsed,
            file_set.total_bytes,
        );
        print_total(
            "gpu_xxh3_total_copy_prepare_hash",
            read + prepare + gpu_xxh3_copy.hash,
            file_set.total_bytes,
        );
        print_total(
            "gpu_xxh3_total_direct_all_read_hash",
            direct_prepare + gpu_xxh3_direct.hash,
            file_set.total_bytes,
        );
        print_total(
            "hybrid_xxh3_total_read_hash",
            hybrid_input.wall + hybrid_xxh3.wall,
            file_set.total_bytes,
        );
        print_speedup(
            "xxh3_copy_total_speedup",
            read + cpu_xxh3.elapsed,
            read + prepare + gpu_xxh3_copy.hash,
        );
        print_speedup(
            "xxh3_direct_all_total_speedup",
            read + cpu_xxh3.elapsed,
            direct_prepare + gpu_xxh3_direct.hash,
        );
        print_speedup(
            "xxh3_hybrid_total_speedup",
            read + cpu_xxh3.elapsed,
            hybrid_input.wall + hybrid_xxh3.wall,
        );

        let cpu_sha256 = cpu_sha256(&messages);
        let gpu_sha256_copy = gpu_sha256(&gpu, &batch)?;
        if cpu_sha256.output != gpu_sha256_copy.output
            || cpu_sha256.output != gpu_sha256_direct.output
        {
            return Err(gpuhash::GpuHashError::Metal(
                "SHA-256 CPU/GPU output mismatch".to_owned(),
            ));
        }
        verify_partitioned(
            &cpu_sha256.output,
            &hybrid.gpu_indices,
            &hybrid_sha256.small_output,
            &hybrid.cpu_indices,
            &hybrid_sha256.large_output,
            "SHA-256 hybrid output mismatch",
        )?;
        print_phase(
            "cpu_sha256_hash_buffers",
            cpu_sha256.elapsed,
            file_set.total_bytes,
        );
        print_phase(
            "gpu_sha256_hash_copy_prepared",
            gpu_sha256_copy.hash,
            file_set.total_bytes,
        );
        print_phase(
            "gpu_sha256_hash_direct_all_prepared",
            gpu_sha256_direct.hash,
            file_set.total_bytes,
        );
        print_total(
            "cpu_sha256_total_read_hash",
            read + cpu_sha256.elapsed,
            file_set.total_bytes,
        );
        print_total(
            "gpu_sha256_total_copy_prepare_hash",
            read + prepare + gpu_sha256_copy.hash,
            file_set.total_bytes,
        );
        print_total(
            "gpu_sha256_total_direct_all_read_hash",
            direct_prepare + gpu_sha256_direct.hash,
            file_set.total_bytes,
        );
        print_total(
            "hybrid_sha256_total_read_hash",
            hybrid_input.wall + hybrid_sha256.wall,
            file_set.total_bytes,
        );
        print_speedup(
            "sha256_copy_total_speedup",
            read + cpu_sha256.elapsed,
            read + prepare + gpu_sha256_copy.hash,
        );
        print_speedup(
            "sha256_direct_all_total_speedup",
            read + cpu_sha256.elapsed,
            direct_prepare + gpu_sha256_direct.hash,
        );
        print_speedup(
            "sha256_hybrid_total_speedup",
            read + cpu_sha256.elapsed,
            hybrid_input.wall + hybrid_sha256.wall,
        );
    }

    Ok(())
}

fn parse_args() -> gpuhash::Result<Args> {
    let mut root = env::var_os("GPUHASH_SMALL_FILES_ROOT").map(PathBuf::from);
    let mut repetitions = env::var("GPUHASH_SMALL_FILES_REPETITIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    let mut gpu_max_file_bytes = env::var("GPUHASH_SMALL_FILES_GPU_MAX_BYTES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_HYBRID_GPU_MAX_FILE_BYTES);

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bench" => {}
            "--root" => {
                let value = args.next().ok_or_else(|| {
                    gpuhash::GpuHashError::Metal("--root requires a path".to_owned())
                })?;
                root = Some(PathBuf::from(value));
            }
            "--repetitions" => {
                let value = args.next().ok_or_else(|| {
                    gpuhash::GpuHashError::Metal("--repetitions requires a number".to_owned())
                })?;
                repetitions = value.parse().map_err(|err| {
                    gpuhash::GpuHashError::Metal(format!("invalid --repetitions: {err}"))
                })?;
            }
            "--gpu-max-file-bytes" => {
                let value = args.next().ok_or_else(|| {
                    gpuhash::GpuHashError::Metal(
                        "--gpu-max-file-bytes requires a number".to_owned(),
                    )
                })?;
                gpu_max_file_bytes = value.parse().map_err(|err| {
                    gpuhash::GpuHashError::Metal(format!("invalid --gpu-max-file-bytes: {err}"))
                })?;
            }
            "--help" | "-h" => {
                println!(
                    "Usage: cargo bench --bench small_files -- --root <dir> [--repetitions N] [--gpu-max-file-bytes N]"
                );
                std::process::exit(0);
            }
            other => {
                return Err(gpuhash::GpuHashError::Metal(format!(
                    "unknown argument: {other}"
                )));
            }
        }
    }

    let root = root.ok_or_else(|| {
        gpuhash::GpuHashError::Metal(
            "small-files benchmark requires --root or GPUHASH_SMALL_FILES_ROOT".to_owned(),
        )
    })?;

    if repetitions == 0 {
        repetitions = 1;
    }

    Ok(Args {
        root,
        repetitions,
        gpu_max_file_bytes,
    })
}

fn scan_files(root: &Path) -> gpuhash::Result<FileSet> {
    let started = Instant::now();
    let mut stack = vec![root.to_path_buf()];
    let mut files = Vec::new();
    let mut total_bytes = 0u64;
    let mut skipped_dirs = 0usize;
    let mut skipped_symlinks = 0usize;

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).map_err(|err| io_error("read_dir", &dir, err))? {
            let entry = entry.map_err(|err| io_error("read_dir entry", &dir, err))?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|err| io_error("file_type", &path, err))?;

            if file_type.is_dir() {
                if entry.file_name() == ".git" {
                    skipped_dirs += 1;
                } else {
                    stack.push(path);
                }
            } else if file_type.is_file() {
                let len = entry
                    .metadata()
                    .map_err(|err| io_error("metadata", &path, err))?
                    .len();
                total_bytes += len;
                files.push(FileEntry { path, len });
            } else if file_type.is_symlink() {
                skipped_symlinks += 1;
            }
        }
    }

    files.sort_unstable_by(|a, b| a.path.cmp(&b.path));

    Ok(FileSet {
        files,
        total_bytes,
        skipped_dirs,
        skipped_symlinks,
        scan_elapsed: started.elapsed(),
    })
}

fn partition_for_hybrid(files: &[FileEntry], gpu_max_file_bytes: u64) -> HybridPartition {
    let mut gpu_files = Vec::new();
    let mut gpu_indices = Vec::new();
    let mut gpu_bytes = 0u64;
    let mut cpu_files = Vec::new();
    let mut cpu_indices = Vec::new();
    let mut cpu_bytes = 0u64;

    for (idx, file) in files.iter().enumerate() {
        if file.len <= gpu_max_file_bytes {
            gpu_bytes += file.len;
            gpu_files.push(file.clone());
            gpu_indices.push(idx);
        } else {
            cpu_bytes += file.len;
            cpu_files.push(file.clone());
            cpu_indices.push(idx);
        }
    }

    HybridPartition {
        gpu_files,
        gpu_indices,
        gpu_bytes,
        cpu_files,
        cpu_indices,
        cpu_bytes,
    }
}

fn prepare_hybrid_input(
    builder: &mut PreparedBatchBuilder,
    partition: &HybridPartition,
) -> gpuhash::Result<HybridInput> {
    let wall_start = Instant::now();

    let (small_batch, large_read, small_prepare) = thread::scope(|scope| {
        let large_handle = scope.spawn(|| read_all_timed(&partition.cpu_files));

        let small_start = Instant::now();
        let small_batch = prepare_files_direct(builder, &partition.gpu_files)?;
        let small_prepare = small_start.elapsed();

        let large_read = large_handle
            .join()
            .map_err(|_| gpuhash::GpuHashError::Metal("large file reader panicked".to_owned()))??;

        Ok::<_, gpuhash::GpuHashError>((small_batch, large_read, small_prepare))
    })?;

    Ok(HybridInput {
        small_batch,
        large_messages: large_read.output,
        small_prepare,
        large_read: large_read.elapsed,
        wall: wall_start.elapsed(),
    })
}

fn hybrid_xxh3(
    gpu: &GpuHash,
    small_batch: &gpuhash::PreparedBatch,
    large_messages: &[Vec<u8>],
) -> gpuhash::Result<HybridHashRun<u64>> {
    let wall_start = Instant::now();

    let (small, large) = thread::scope(|scope| {
        let large_handle = scope.spawn(|| cpu_xxh3(large_messages));
        let small = gpu_xxh3(gpu, small_batch)?;
        let large = large_handle
            .join()
            .map_err(|_| gpuhash::GpuHashError::Metal("large XXH3 worker panicked".to_owned()))?;

        Ok::<_, gpuhash::GpuHashError>((small, large))
    })?;

    Ok(HybridHashRun {
        small_output: small.output,
        large_output: large.output,
        small_hash: small.hash,
        large_hash: large.elapsed,
        wall: wall_start.elapsed(),
    })
}

fn hybrid_sha256(
    gpu: &GpuHash,
    small_batch: &gpuhash::PreparedBatch,
    large_messages: &[Vec<u8>],
) -> gpuhash::Result<HybridHashRun<[u8; 32]>> {
    let wall_start = Instant::now();

    let (small, large) = thread::scope(|scope| {
        let large_handle = scope.spawn(|| cpu_sha256(large_messages));
        let small = gpu_sha256(gpu, small_batch)?;
        let large = large_handle.join().map_err(|_| {
            gpuhash::GpuHashError::Metal("large SHA-256 worker panicked".to_owned())
        })?;

        Ok::<_, gpuhash::GpuHashError>((small, large))
    })?;

    Ok(HybridHashRun {
        small_output: small.output,
        large_output: large.output,
        small_hash: small.hash,
        large_hash: large.elapsed,
        wall: wall_start.elapsed(),
    })
}

fn verify_partitioned<T: Eq>(
    expected: &[T],
    gpu_indices: &[usize],
    gpu_output: &[T],
    cpu_indices: &[usize],
    cpu_output: &[T],
    message: &str,
) -> gpuhash::Result<()> {
    if gpu_indices.len() != gpu_output.len() || cpu_indices.len() != cpu_output.len() {
        return Err(gpuhash::GpuHashError::Metal(format!(
            "{message}: partition/output length mismatch"
        )));
    }

    for (&idx, got) in gpu_indices.iter().zip(gpu_output) {
        if expected.get(idx) != Some(got) {
            return Err(gpuhash::GpuHashError::Metal(format!(
                "{message}: GPU partition mismatch at file index {idx}"
            )));
        }
    }

    for (&idx, got) in cpu_indices.iter().zip(cpu_output) {
        if expected.get(idx) != Some(got) {
            return Err(gpuhash::GpuHashError::Metal(format!(
                "{message}: CPU partition mismatch at file index {idx}"
            )));
        }
    }

    Ok(())
}

fn cpu_xxh3(messages: &[Vec<u8>]) -> CpuRun<u64> {
    let started = Instant::now();
    let mut output = Vec::with_capacity(messages.len());
    for bytes in messages {
        output.push(XxHash3_64::oneshot(bytes));
    }
    CpuRun {
        output,
        elapsed: started.elapsed(),
    }
}

fn cpu_sha256(messages: &[Vec<u8>]) -> CpuRun<[u8; 32]> {
    let started = Instant::now();
    let mut output = Vec::with_capacity(messages.len());
    for bytes in messages {
        output.push(Sha256::digest(bytes).into());
    }
    CpuRun {
        output,
        elapsed: started.elapsed(),
    }
}

fn gpu_xxh3(gpu: &GpuHash, batch: &gpuhash::PreparedBatch) -> gpuhash::Result<GpuHashRun<u64>> {
    let hash_start = Instant::now();
    let output = gpu.xxhash3_64_prepared(batch)?;
    let hash = hash_start.elapsed();

    Ok(GpuHashRun { output, hash })
}

fn gpu_sha256(
    gpu: &GpuHash,
    batch: &gpuhash::PreparedBatch,
) -> gpuhash::Result<GpuHashRun<[u8; 32]>> {
    let hash_start = Instant::now();
    let output = gpu.sha256_prepared(batch)?;
    let hash = hash_start.elapsed();

    Ok(GpuHashRun { output, hash })
}

fn prepare_files_direct(
    builder: &mut PreparedBatchBuilder,
    files: &[FileEntry],
) -> gpuhash::Result<gpuhash::PreparedBatch> {
    let lengths: Vec<_> = files
        .iter()
        .map(|file| {
            usize::try_from(file.len).map_err(|_| {
                gpuhash::GpuHashError::Metal(format!(
                    "file is too large for this platform: {}",
                    file.path.display()
                ))
            })
        })
        .collect::<gpuhash::Result<_>>()?;

    builder.prepare_with_lengths_parallel(&lengths, |idx, dst| {
        let mut file =
            File::open(&files[idx].path).map_err(|err| io_error("open", &files[idx].path, err))?;
        file.read_exact(dst)
            .map_err(|err| io_error("read_exact", &files[idx].path, err))
    })
}

fn read_all(files: &[FileEntry]) -> gpuhash::Result<Vec<Vec<u8>>> {
    files.iter().map(read_file).collect()
}

fn read_all_timed(files: &[FileEntry]) -> gpuhash::Result<Timed<Vec<Vec<u8>>>> {
    let started = Instant::now();
    let output = read_all(files)?;
    Ok(Timed {
        output,
        elapsed: started.elapsed(),
    })
}

fn read_file(file: &FileEntry) -> gpuhash::Result<Vec<u8>> {
    let bytes = fs::read(&file.path).map_err(|err| io_error("read", &file.path, err))?;
    if bytes.len() as u64 != file.len {
        return Err(gpuhash::GpuHashError::Metal(format!(
            "file changed while benchmarking: {} expected {} bytes, read {} bytes",
            file.path.display(),
            file.len,
            bytes.len()
        )));
    }
    Ok(bytes)
}

fn io_error(op: &str, path: &Path, err: io::Error) -> gpuhash::GpuHashError {
    gpuhash::GpuHashError::Metal(format!("{op} failed for {}: {err}", path.display()))
}

fn print_phase(name: &str, elapsed: Duration, bytes: u64) {
    println!(
        "{name:32} total {:>10.3} ms  {:>8.2} MiB/s",
        ms(elapsed),
        mib_per_sec(bytes, elapsed)
    );
}

fn print_total(name: &str, elapsed: Duration, bytes: u64) {
    print_phase(name, elapsed, bytes);
}

fn print_speedup(name: &str, cpu: Duration, gpu: Duration) {
    println!("{name:32} {:>8.3}x", cpu.as_secs_f64() / gpu.as_secs_f64());
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn mib_per_sec(bytes: u64, duration: Duration) -> f64 {
    bytes as f64 / (1024.0 * 1024.0) / duration.as_secs_f64()
}
