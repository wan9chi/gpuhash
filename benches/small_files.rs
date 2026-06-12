use gpuhash::GpuHash;
use sha2::{Digest as _, Sha256};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use twox_hash::XxHash3_64;

#[derive(Debug)]
struct Args {
    root: PathBuf,
    repetitions: usize,
}

#[derive(Debug)]
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

    let gpu_start = Instant::now();
    let gpu = GpuHash::new()?;
    println!("metal init: {:.3} ms", ms(gpu_start.elapsed()));

    for rep in 0..args.repetitions {
        println!();
        println!("repetition: {}/{}", rep + 1, args.repetitions);

        let read_start = Instant::now();
        let messages = read_all(&file_set.files)?;
        let read = read_start.elapsed();
        print_phase("read_all_files", read, file_set.total_bytes);

        let prepare_start = Instant::now();
        let batch = gpu.prepare_batch(&messages)?;
        let prepare = prepare_start.elapsed();
        print_phase("gpu_prepare_batch", prepare, file_set.total_bytes);

        let cpu_xxh3 = cpu_xxh3(&messages);
        let gpu_xxh3 = gpu_xxh3(&gpu, &batch)?;
        if cpu_xxh3.output != gpu_xxh3.output {
            return Err(gpuhash::GpuHashError::Metal(
                "XXH3 CPU/GPU output mismatch".to_owned(),
            ));
        }
        print_phase(
            "cpu_xxh3_hash_buffers",
            cpu_xxh3.elapsed,
            file_set.total_bytes,
        );
        print_phase(
            "gpu_xxh3_hash_prepared",
            gpu_xxh3.hash,
            file_set.total_bytes,
        );
        print_total(
            "cpu_xxh3_total_read_hash",
            read + cpu_xxh3.elapsed,
            file_set.total_bytes,
        );
        print_total(
            "gpu_xxh3_total_read_prepare_hash",
            read + prepare + gpu_xxh3.hash,
            file_set.total_bytes,
        );
        print_speedup(
            "xxh3_total_speedup",
            read + cpu_xxh3.elapsed,
            read + prepare + gpu_xxh3.hash,
        );

        let cpu_sha256 = cpu_sha256(&messages);
        let gpu_sha256 = gpu_sha256(&gpu, &batch)?;
        if cpu_sha256.output != gpu_sha256.output {
            return Err(gpuhash::GpuHashError::Metal(
                "SHA-256 CPU/GPU output mismatch".to_owned(),
            ));
        }
        print_phase(
            "cpu_sha256_hash_buffers",
            cpu_sha256.elapsed,
            file_set.total_bytes,
        );
        print_phase(
            "gpu_sha256_hash_prepared",
            gpu_sha256.hash,
            file_set.total_bytes,
        );
        print_total(
            "cpu_sha256_total_read_hash",
            read + cpu_sha256.elapsed,
            file_set.total_bytes,
        );
        print_total(
            "gpu_sha256_total_read_prepare_hash",
            read + prepare + gpu_sha256.hash,
            file_set.total_bytes,
        );
        print_speedup(
            "sha256_total_speedup",
            read + cpu_sha256.elapsed,
            read + prepare + gpu_sha256.hash,
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
            "--help" | "-h" => {
                println!(
                    "Usage: cargo bench --bench small_files -- --root <dir> [--repetitions N]"
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

    Ok(Args { root, repetitions })
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

fn read_all(files: &[FileEntry]) -> gpuhash::Result<Vec<Vec<u8>>> {
    files.iter().map(read_file).collect()
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
