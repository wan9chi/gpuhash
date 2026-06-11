#![forbid(unsafe_op_in_unsafe_fn)]

#[cfg(target_os = "macos")]
mod metal_backend;

use std::{error::Error, fmt, hash};

/// Result type used by this crate.
pub type Result<T> = std::result::Result<T, GpuHashError>;

/// Minimum byte length for XXH3 custom secrets.
pub const XXH3_SECRET_MINIMUM_LENGTH: usize = 136;

/// Errors returned by the GPU backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuHashError {
    /// The current platform does not expose a usable Metal device.
    MetalUnavailable,
    /// A Metal API or shader compilation call failed.
    Metal(String),
    /// The supplied XXH3 custom secret is too short.
    SecretTooShort { minimum: usize, actual: usize },
    /// The requested input is too large for this API's Metal buffer layout.
    InputTooLarge,
}

impl fmt::Display for GpuHashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MetalUnavailable => write!(f, "no usable Apple Metal device is available"),
            Self::Metal(message) => write!(f, "Metal error: {message}"),
            Self::SecretTooShort { minimum, actual } => {
                write!(
                    f,
                    "XXH3 secret is too short: got {actual} bytes, need at least {minimum}"
                )
            }
            Self::InputTooLarge => write!(f, "input is too large for the GPU batch layout"),
        }
    }
}

impl Error for GpuHashError {}

fn validate_xxh3_secret(secret: &[u8]) -> Result<()> {
    if secret.len() < XXH3_SECRET_MINIMUM_LENGTH {
        return Err(GpuHashError::SecretTooShort {
            minimum: XXH3_SECRET_MINIMUM_LENGTH,
            actual: secret.len(),
        });
    }

    Ok(())
}

/// A reusable batch of messages stored in Apple Silicon shared memory.
///
/// Preparing a batch does the CPU-side packing once. Hashing the prepared batch
/// dispatches a Metal compute kernel over the already shared buffers.
pub struct PreparedBatch {
    #[cfg(target_os = "macos")]
    inner: metal_backend::PreparedBatch,
    count: usize,
    total_bytes: usize,
}

impl PreparedBatch {
    /// Number of messages in the batch.
    #[must_use]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns true when the batch contains no messages.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Sum of all unpadded message lengths.
    #[must_use]
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }
}

/// Apple Silicon GPU hash engine.
pub struct GpuHash {
    #[cfg(target_os = "macos")]
    inner: metal_backend::GpuHash,
}

impl GpuHash {
    /// Creates a Metal-backed hash engine.
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            Ok(Self {
                inner: metal_backend::GpuHash::new()?,
            })
        }

        #[cfg(not(target_os = "macos"))]
        {
            Err(GpuHashError::MetalUnavailable)
        }
    }

    /// Returns whether a GPU hash engine can be created on this machine.
    #[must_use]
    pub fn is_available() -> bool {
        Self::new().is_ok()
    }

    /// Packs messages into GPU-visible shared memory for repeated hashing.
    pub fn prepare_batch<M: AsRef<[u8]>>(&self, messages: &[M]) -> Result<PreparedBatch> {
        #[cfg(target_os = "macos")]
        {
            let inner = self.inner.prepare_batch(messages)?;
            Ok(PreparedBatch {
                count: inner.count(),
                total_bytes: inner.total_bytes(),
                inner,
            })
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = messages;
            Err(GpuHashError::MetalUnavailable)
        }
    }

    /// Computes `twox_hash::XxHash32::oneshot(seed, message)` for every message.
    pub fn xxhash32_prepared(&self, seed: u32, batch: &PreparedBatch) -> Result<Vec<u32>> {
        #[cfg(target_os = "macos")]
        {
            self.inner.xxhash32(seed, &batch.inner)
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (seed, batch);
            Err(GpuHashError::MetalUnavailable)
        }
    }

    /// Computes `twox_hash::XxHash64::oneshot(seed, message)` for every message.
    pub fn xxhash64_prepared(&self, seed: u64, batch: &PreparedBatch) -> Result<Vec<u64>> {
        #[cfg(target_os = "macos")]
        {
            self.inner.xxhash64(seed, &batch.inner)
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (seed, batch);
            Err(GpuHashError::MetalUnavailable)
        }
    }

    /// Computes `twox_hash::XxHash3_64::oneshot(message)` for every message.
    pub fn xxhash3_64_prepared(&self, batch: &PreparedBatch) -> Result<Vec<u64>> {
        self.xxhash3_64_with_seed_prepared(0, batch)
    }

    /// Computes `twox_hash::XxHash3_64::oneshot_with_seed(seed, message)` for every message.
    pub fn xxhash3_64_with_seed_prepared(
        &self,
        seed: u64,
        batch: &PreparedBatch,
    ) -> Result<Vec<u64>> {
        #[cfg(target_os = "macos")]
        {
            self.inner.xxhash3_64(seed, &batch.inner)
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (seed, batch);
            Err(GpuHashError::MetalUnavailable)
        }
    }

    /// Computes `twox_hash::XxHash3_64::oneshot_with_secret(secret, message)`
    /// for every message.
    pub fn xxhash3_64_with_secret_prepared(
        &self,
        secret: &[u8],
        batch: &PreparedBatch,
    ) -> Result<Vec<u64>> {
        #[cfg(target_os = "macos")]
        {
            self.inner.xxhash3_64_with_secret(secret, &batch.inner)
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (secret, batch);
            Err(GpuHashError::MetalUnavailable)
        }
    }

    /// Computes `twox_hash::XxHash3_64::oneshot_with_seed_and_secret(seed, secret, message)`
    /// for every message.
    pub fn xxhash3_64_with_seed_and_secret_prepared(
        &self,
        seed: u64,
        secret: &[u8],
        batch: &PreparedBatch,
    ) -> Result<Vec<u64>> {
        #[cfg(target_os = "macos")]
        {
            self.inner
                .xxhash3_64_with_seed_and_secret(seed, secret, &batch.inner)
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (seed, secret, batch);
            Err(GpuHashError::MetalUnavailable)
        }
    }

    /// Computes `twox_hash::XxHash3_128::oneshot(message)` for every message.
    pub fn xxhash3_128_prepared(&self, batch: &PreparedBatch) -> Result<Vec<u128>> {
        self.xxhash3_128_with_seed_prepared(0, batch)
    }

    /// Computes `twox_hash::XxHash3_128::oneshot_with_seed(seed, message)` for every message.
    pub fn xxhash3_128_with_seed_prepared(
        &self,
        seed: u64,
        batch: &PreparedBatch,
    ) -> Result<Vec<u128>> {
        #[cfg(target_os = "macos")]
        {
            self.inner.xxhash3_128(seed, &batch.inner)
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (seed, batch);
            Err(GpuHashError::MetalUnavailable)
        }
    }

    /// Computes `twox_hash::XxHash3_128::oneshot_with_secret(secret, message)`
    /// for every message.
    pub fn xxhash3_128_with_secret_prepared(
        &self,
        secret: &[u8],
        batch: &PreparedBatch,
    ) -> Result<Vec<u128>> {
        #[cfg(target_os = "macos")]
        {
            self.inner.xxhash3_128_with_secret(secret, &batch.inner)
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (secret, batch);
            Err(GpuHashError::MetalUnavailable)
        }
    }

    /// Computes `twox_hash::XxHash3_128::oneshot_with_seed_and_secret(seed, secret, message)`
    /// for every message.
    pub fn xxhash3_128_with_seed_and_secret_prepared(
        &self,
        seed: u64,
        secret: &[u8],
        batch: &PreparedBatch,
    ) -> Result<Vec<u128>> {
        #[cfg(target_os = "macos")]
        {
            self.inner
                .xxhash3_128_with_seed_and_secret(seed, secret, &batch.inner)
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (seed, secret, batch);
            Err(GpuHashError::MetalUnavailable)
        }
    }

    /// Computes RustCrypto-compatible SHA-256 digests for every message.
    pub fn sha256_prepared(&self, batch: &PreparedBatch) -> Result<Vec<[u8; 32]>> {
        #[cfg(target_os = "macos")]
        {
            self.inner.sha256(&batch.inner)
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = batch;
            Err(GpuHashError::MetalUnavailable)
        }
    }

    /// Convenience wrapper that prepares and hashes a batch with xxHash32.
    pub fn xxhash32<M: AsRef<[u8]>>(&self, seed: u32, messages: &[M]) -> Result<Vec<u32>> {
        let batch = self.prepare_batch(messages)?;
        self.xxhash32_prepared(seed, &batch)
    }

    /// Convenience wrapper that prepares and hashes a batch with xxHash64.
    pub fn xxhash64<M: AsRef<[u8]>>(&self, seed: u64, messages: &[M]) -> Result<Vec<u64>> {
        let batch = self.prepare_batch(messages)?;
        self.xxhash64_prepared(seed, &batch)
    }

    /// Convenience wrapper that prepares and hashes a batch with XXH3-64.
    pub fn xxhash3_64<M: AsRef<[u8]>>(&self, messages: &[M]) -> Result<Vec<u64>> {
        let batch = self.prepare_batch(messages)?;
        self.xxhash3_64_prepared(&batch)
    }

    /// Convenience wrapper that prepares and hashes a batch with seeded XXH3-64.
    pub fn xxhash3_64_with_seed<M: AsRef<[u8]>>(
        &self,
        seed: u64,
        messages: &[M],
    ) -> Result<Vec<u64>> {
        let batch = self.prepare_batch(messages)?;
        self.xxhash3_64_with_seed_prepared(seed, &batch)
    }

    /// Convenience wrapper that prepares and hashes a batch with custom-secret
    /// XXH3-64.
    pub fn xxhash3_64_with_secret<M: AsRef<[u8]>>(
        &self,
        secret: &[u8],
        messages: &[M],
    ) -> Result<Vec<u64>> {
        let batch = self.prepare_batch(messages)?;
        self.xxhash3_64_with_secret_prepared(secret, &batch)
    }

    /// Convenience wrapper that prepares and hashes a batch with seeded,
    /// custom-secret XXH3-64.
    pub fn xxhash3_64_with_seed_and_secret<M: AsRef<[u8]>>(
        &self,
        seed: u64,
        secret: &[u8],
        messages: &[M],
    ) -> Result<Vec<u64>> {
        let batch = self.prepare_batch(messages)?;
        self.xxhash3_64_with_seed_and_secret_prepared(seed, secret, &batch)
    }

    /// Convenience wrapper that prepares and hashes a batch with XXH3-128.
    pub fn xxhash3_128<M: AsRef<[u8]>>(&self, messages: &[M]) -> Result<Vec<u128>> {
        let batch = self.prepare_batch(messages)?;
        self.xxhash3_128_prepared(&batch)
    }

    /// Convenience wrapper that prepares and hashes a batch with seeded XXH3-128.
    pub fn xxhash3_128_with_seed<M: AsRef<[u8]>>(
        &self,
        seed: u64,
        messages: &[M],
    ) -> Result<Vec<u128>> {
        let batch = self.prepare_batch(messages)?;
        self.xxhash3_128_with_seed_prepared(seed, &batch)
    }

    /// Convenience wrapper that prepares and hashes a batch with custom-secret
    /// XXH3-128.
    pub fn xxhash3_128_with_secret<M: AsRef<[u8]>>(
        &self,
        secret: &[u8],
        messages: &[M],
    ) -> Result<Vec<u128>> {
        let batch = self.prepare_batch(messages)?;
        self.xxhash3_128_with_secret_prepared(secret, &batch)
    }

    /// Convenience wrapper that prepares and hashes a batch with seeded,
    /// custom-secret XXH3-128.
    pub fn xxhash3_128_with_seed_and_secret<M: AsRef<[u8]>>(
        &self,
        seed: u64,
        secret: &[u8],
        messages: &[M],
    ) -> Result<Vec<u128>> {
        let batch = self.prepare_batch(messages)?;
        self.xxhash3_128_with_seed_and_secret_prepared(seed, secret, &batch)
    }

    /// Convenience wrapper that prepares and hashes a batch with SHA-256.
    pub fn sha256<M: AsRef<[u8]>>(&self, messages: &[M]) -> Result<Vec<[u8; 32]>> {
        let batch = self.prepare_batch(messages)?;
        self.sha256_prepared(&batch)
    }
}

/// Convenience wrapper around [`GpuHash::xxhash32`].
pub fn xxhash32_batch<M: AsRef<[u8]>>(seed: u32, messages: &[M]) -> Result<Vec<u32>> {
    GpuHash::new()?.xxhash32(seed, messages)
}

/// Convenience wrapper around [`GpuHash::xxhash64`].
pub fn xxhash64_batch<M: AsRef<[u8]>>(seed: u64, messages: &[M]) -> Result<Vec<u64>> {
    GpuHash::new()?.xxhash64(seed, messages)
}

/// Convenience wrapper around [`GpuHash::xxhash3_64`].
pub fn xxhash3_64_batch<M: AsRef<[u8]>>(messages: &[M]) -> Result<Vec<u64>> {
    GpuHash::new()?.xxhash3_64(messages)
}

/// Convenience wrapper around [`GpuHash::xxhash3_64_with_seed`].
pub fn xxhash3_64_with_seed_batch<M: AsRef<[u8]>>(seed: u64, messages: &[M]) -> Result<Vec<u64>> {
    GpuHash::new()?.xxhash3_64_with_seed(seed, messages)
}

/// Convenience wrapper around [`GpuHash::xxhash3_64_with_secret`].
pub fn xxhash3_64_with_secret_batch<M: AsRef<[u8]>>(
    secret: &[u8],
    messages: &[M],
) -> Result<Vec<u64>> {
    GpuHash::new()?.xxhash3_64_with_secret(secret, messages)
}

/// Convenience wrapper around [`GpuHash::xxhash3_64_with_seed_and_secret`].
pub fn xxhash3_64_with_seed_and_secret_batch<M: AsRef<[u8]>>(
    seed: u64,
    secret: &[u8],
    messages: &[M],
) -> Result<Vec<u64>> {
    GpuHash::new()?.xxhash3_64_with_seed_and_secret(seed, secret, messages)
}

/// Convenience wrapper around [`GpuHash::xxhash3_128`].
pub fn xxhash3_128_batch<M: AsRef<[u8]>>(messages: &[M]) -> Result<Vec<u128>> {
    GpuHash::new()?.xxhash3_128(messages)
}

/// Convenience wrapper around [`GpuHash::xxhash3_128_with_seed`].
pub fn xxhash3_128_with_seed_batch<M: AsRef<[u8]>>(seed: u64, messages: &[M]) -> Result<Vec<u128>> {
    GpuHash::new()?.xxhash3_128_with_seed(seed, messages)
}

/// Convenience wrapper around [`GpuHash::xxhash3_128_with_secret`].
pub fn xxhash3_128_with_secret_batch<M: AsRef<[u8]>>(
    secret: &[u8],
    messages: &[M],
) -> Result<Vec<u128>> {
    GpuHash::new()?.xxhash3_128_with_secret(secret, messages)
}

/// Convenience wrapper around [`GpuHash::xxhash3_128_with_seed_and_secret`].
pub fn xxhash3_128_with_seed_and_secret_batch<M: AsRef<[u8]>>(
    seed: u64,
    secret: &[u8],
    messages: &[M],
) -> Result<Vec<u128>> {
    GpuHash::new()?.xxhash3_128_with_seed_and_secret(seed, secret, messages)
}

/// Convenience wrapper around [`GpuHash::sha256`].
pub fn sha256_batch<M: AsRef<[u8]>>(messages: &[M]) -> Result<Vec<[u8; 32]>> {
    GpuHash::new()?.sha256(messages)
}

/// Buffered GPU-backed xxHash32 hasher.
///
/// This compatibility surface collects streaming writes and finalizes with the
/// Metal one-shot kernel. Use [`GpuHash::xxhash32_prepared`] for high-throughput
/// batches.
pub struct GpuXxHash32 {
    engine: GpuHash,
    seed: u32,
    bytes: Vec<u8>,
}

impl GpuXxHash32 {
    /// Constructs a hasher with seed 0.
    pub fn new() -> Result<Self> {
        Self::with_seed(0)
    }

    /// Constructs a hasher with an initial seed.
    pub fn with_seed(seed: u32) -> Result<Self> {
        Ok(Self {
            engine: GpuHash::new()?,
            seed,
            bytes: Vec::new(),
        })
    }

    /// The seed this hasher was created with.
    #[must_use]
    pub fn seed(&self) -> u32 {
        self.seed
    }

    /// The total number of bytes written.
    #[must_use]
    pub fn total_len(&self) -> u64 {
        self.bytes.len() as u64
    }

    /// Writes more bytes into this hasher.
    pub fn write(&mut self, input: &[u8]) {
        self.bytes.extend_from_slice(input);
    }

    /// Returns the current 32-bit hash value.
    pub fn finish_32(&self) -> Result<u32> {
        let messages = [self.bytes.as_slice()];
        Ok(self.engine.xxhash32(self.seed, &messages)?[0])
    }
}

impl Default for GpuXxHash32 {
    fn default() -> Self {
        Self::new().expect("failed to create default GPU xxHash32 hasher")
    }
}

impl hash::Hasher for GpuXxHash32 {
    fn write(&mut self, bytes: &[u8]) {
        Self::write(self, bytes);
    }

    fn finish(&self) -> u64 {
        self.finish_32()
            .expect("failed to finish GPU xxHash32 hasher") as u64
    }
}

/// Constructs [`GpuXxHash32`] instances with a fixed seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuXxHash32State(u32);

impl GpuXxHash32State {
    /// Constructs a builder with an initial seed.
    #[must_use]
    pub const fn with_seed(seed: u32) -> Self {
        Self(seed)
    }

    /// The seed used for hashers created by this builder.
    #[must_use]
    pub const fn seed(&self) -> u32 {
        self.0
    }
}

impl Default for GpuXxHash32State {
    fn default() -> Self {
        Self::with_seed(0)
    }
}

impl hash::BuildHasher for GpuXxHash32State {
    type Hasher = GpuXxHash32;

    fn build_hasher(&self) -> Self::Hasher {
        GpuXxHash32::with_seed(self.0).expect("failed to create GPU xxHash32 hasher")
    }
}

/// Buffered GPU-backed xxHash64 hasher.
pub struct GpuXxHash64 {
    engine: GpuHash,
    seed: u64,
    bytes: Vec<u8>,
}

impl GpuXxHash64 {
    /// Constructs a hasher with seed 0.
    pub fn new() -> Result<Self> {
        Self::with_seed(0)
    }

    /// Constructs a hasher with an initial seed.
    pub fn with_seed(seed: u64) -> Result<Self> {
        Ok(Self {
            engine: GpuHash::new()?,
            seed,
            bytes: Vec::new(),
        })
    }

    /// The seed this hasher was created with.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// The total number of bytes written.
    #[must_use]
    pub fn total_len(&self) -> u64 {
        self.bytes.len() as u64
    }

    /// Writes more bytes into this hasher.
    pub fn write(&mut self, input: &[u8]) {
        self.bytes.extend_from_slice(input);
    }

    /// Returns the current 64-bit hash value.
    pub fn finish_64(&self) -> Result<u64> {
        let messages = [self.bytes.as_slice()];
        Ok(self.engine.xxhash64(self.seed, &messages)?[0])
    }
}

impl Default for GpuXxHash64 {
    fn default() -> Self {
        Self::new().expect("failed to create default GPU xxHash64 hasher")
    }
}

impl hash::Hasher for GpuXxHash64 {
    fn write(&mut self, bytes: &[u8]) {
        Self::write(self, bytes);
    }

    fn finish(&self) -> u64 {
        self.finish_64()
            .expect("failed to finish GPU xxHash64 hasher")
    }
}

/// Constructs [`GpuXxHash64`] instances with a fixed seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuXxHash64State(u64);

impl GpuXxHash64State {
    /// Constructs a builder with an initial seed.
    #[must_use]
    pub const fn with_seed(seed: u64) -> Self {
        Self(seed)
    }

    /// The seed used for hashers created by this builder.
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.0
    }
}

impl Default for GpuXxHash64State {
    fn default() -> Self {
        Self::with_seed(0)
    }
}

impl hash::BuildHasher for GpuXxHash64State {
    type Hasher = GpuXxHash64;

    fn build_hasher(&self) -> Self::Hasher {
        GpuXxHash64::with_seed(self.0).expect("failed to create GPU xxHash64 hasher")
    }
}

enum Xxh3SecretMode {
    Seeded,
    Secret(Vec<u8>),
    SeedAndSecret(Vec<u8>),
}

/// Buffered GPU-backed XXH3-64 hasher.
pub struct GpuXxHash3_64 {
    engine: GpuHash,
    seed: u64,
    secret_mode: Xxh3SecretMode,
    bytes: Vec<u8>,
}

impl GpuXxHash3_64 {
    /// Constructs a hasher with the default seed and secret.
    pub fn new() -> Result<Self> {
        Self::with_seed(0)
    }

    /// Constructs a hasher with a seed-derived secret.
    pub fn with_seed(seed: u64) -> Result<Self> {
        Ok(Self {
            engine: GpuHash::new()?,
            seed,
            secret_mode: Xxh3SecretMode::Seeded,
            bytes: Vec::new(),
        })
    }

    /// Constructs a hasher with the default seed and a custom secret.
    pub fn with_secret(secret: impl Into<Vec<u8>>) -> Result<Self> {
        let secret = secret.into();
        validate_xxh3_secret(&secret)?;
        Ok(Self {
            engine: GpuHash::new()?,
            seed: 0,
            secret_mode: Xxh3SecretMode::Secret(secret),
            bytes: Vec::new(),
        })
    }

    /// Constructs a hasher with a seed and custom secret.
    pub fn with_seed_and_secret(seed: u64, secret: impl Into<Vec<u8>>) -> Result<Self> {
        let secret = secret.into();
        validate_xxh3_secret(&secret)?;
        Ok(Self {
            engine: GpuHash::new()?,
            seed,
            secret_mode: Xxh3SecretMode::SeedAndSecret(secret),
            bytes: Vec::new(),
        })
    }

    /// The total number of bytes written.
    #[must_use]
    pub fn total_len(&self) -> u64 {
        self.bytes.len() as u64
    }

    /// Writes more bytes into this hasher.
    pub fn write(&mut self, input: &[u8]) {
        self.bytes.extend_from_slice(input);
    }

    /// Returns the current 64-bit XXH3 value.
    pub fn finish_64(&self) -> Result<u64> {
        let messages = [self.bytes.as_slice()];
        let output = match &self.secret_mode {
            Xxh3SecretMode::Seeded => self.engine.xxhash3_64_with_seed(self.seed, &messages)?,
            Xxh3SecretMode::Secret(secret) => {
                self.engine.xxhash3_64_with_secret(secret, &messages)?
            }
            Xxh3SecretMode::SeedAndSecret(secret) => self
                .engine
                .xxhash3_64_with_seed_and_secret(self.seed, secret, &messages)?,
        };
        Ok(output[0])
    }
}

impl Default for GpuXxHash3_64 {
    fn default() -> Self {
        Self::new().expect("failed to create default GPU XXH3-64 hasher")
    }
}

impl hash::Hasher for GpuXxHash3_64 {
    fn write(&mut self, bytes: &[u8]) {
        Self::write(self, bytes);
    }

    fn finish(&self) -> u64 {
        self.finish_64()
            .expect("failed to finish GPU XXH3-64 hasher")
    }
}

/// Constructs [`GpuXxHash3_64`] instances with a fixed seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuXxHash3_64State(u64);

impl GpuXxHash3_64State {
    /// Constructs a builder with an initial seed.
    #[must_use]
    pub const fn with_seed(seed: u64) -> Self {
        Self(seed)
    }

    /// The seed used for hashers created by this builder.
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.0
    }
}

impl Default for GpuXxHash3_64State {
    fn default() -> Self {
        Self::with_seed(0)
    }
}

impl hash::BuildHasher for GpuXxHash3_64State {
    type Hasher = GpuXxHash3_64;

    fn build_hasher(&self) -> Self::Hasher {
        GpuXxHash3_64::with_seed(self.0).expect("failed to create GPU XXH3-64 hasher")
    }
}

/// Buffered GPU-backed XXH3-128 hasher.
pub struct GpuXxHash3_128 {
    engine: GpuHash,
    seed: u64,
    secret_mode: Xxh3SecretMode,
    bytes: Vec<u8>,
}

impl GpuXxHash3_128 {
    /// Constructs a hasher with the default seed and secret.
    pub fn new() -> Result<Self> {
        Self::with_seed(0)
    }

    /// Constructs a hasher with a seed-derived secret.
    pub fn with_seed(seed: u64) -> Result<Self> {
        Ok(Self {
            engine: GpuHash::new()?,
            seed,
            secret_mode: Xxh3SecretMode::Seeded,
            bytes: Vec::new(),
        })
    }

    /// Constructs a hasher with the default seed and a custom secret.
    pub fn with_secret(secret: impl Into<Vec<u8>>) -> Result<Self> {
        let secret = secret.into();
        validate_xxh3_secret(&secret)?;
        Ok(Self {
            engine: GpuHash::new()?,
            seed: 0,
            secret_mode: Xxh3SecretMode::Secret(secret),
            bytes: Vec::new(),
        })
    }

    /// Constructs a hasher with a seed and custom secret.
    pub fn with_seed_and_secret(seed: u64, secret: impl Into<Vec<u8>>) -> Result<Self> {
        let secret = secret.into();
        validate_xxh3_secret(&secret)?;
        Ok(Self {
            engine: GpuHash::new()?,
            seed,
            secret_mode: Xxh3SecretMode::SeedAndSecret(secret),
            bytes: Vec::new(),
        })
    }

    /// The total number of bytes written.
    #[must_use]
    pub fn total_len(&self) -> u64 {
        self.bytes.len() as u64
    }

    /// Writes more bytes into this hasher.
    pub fn write(&mut self, input: &[u8]) {
        self.bytes.extend_from_slice(input);
    }

    /// Returns the current 128-bit XXH3 value.
    pub fn finish_128(&self) -> Result<u128> {
        let messages = [self.bytes.as_slice()];
        let output = match &self.secret_mode {
            Xxh3SecretMode::Seeded => self.engine.xxhash3_128_with_seed(self.seed, &messages)?,
            Xxh3SecretMode::Secret(secret) => {
                self.engine.xxhash3_128_with_secret(secret, &messages)?
            }
            Xxh3SecretMode::SeedAndSecret(secret) => self
                .engine
                .xxhash3_128_with_seed_and_secret(self.seed, secret, &messages)?,
        };
        Ok(output[0])
    }
}

impl Default for GpuXxHash3_128 {
    fn default() -> Self {
        Self::new().expect("failed to create default GPU XXH3-128 hasher")
    }
}

/// Buffered GPU-backed SHA-256 hasher.
pub struct GpuSha256 {
    engine: GpuHash,
    bytes: Vec<u8>,
}

impl GpuSha256 {
    /// Fallibly constructs a SHA-256 hasher.
    pub fn try_new() -> Result<Self> {
        Ok(Self {
            engine: GpuHash::new()?,
            bytes: Vec::new(),
        })
    }

    /// The total number of bytes written.
    #[must_use]
    pub fn total_len(&self) -> u64 {
        self.bytes.len() as u64
    }

    /// Writes more bytes into this hasher.
    pub fn write(&mut self, input: &[u8]) {
        self.bytes.extend_from_slice(input);
    }

    /// Fallibly finalizes the current byte stream as a SHA-256 digest.
    pub fn try_finalize(&self) -> Result<[u8; 32]> {
        let messages = [self.bytes.as_slice()];
        Ok(self.engine.sha256(&messages)?[0])
    }
}

impl Default for GpuSha256 {
    fn default() -> Self {
        Self::try_new().expect("failed to create default GPU SHA-256 hasher")
    }
}

impl digest::OutputSizeUser for GpuSha256 {
    type OutputSize = digest::consts::U32;
}

impl digest::Update for GpuSha256 {
    fn update(&mut self, data: &[u8]) {
        self.write(data);
    }
}

impl digest::FixedOutput for GpuSha256 {
    fn finalize_into(self, out: &mut digest::Output<Self>) {
        let digest = self
            .try_finalize()
            .expect("failed to finish GPU SHA-256 digest");
        out.copy_from_slice(&digest);
    }
}

impl digest::FixedOutputReset for GpuSha256 {
    fn finalize_into_reset(&mut self, out: &mut digest::Output<Self>) {
        let digest = self
            .try_finalize()
            .expect("failed to finish GPU SHA-256 digest");
        out.copy_from_slice(&digest);
        self.bytes.clear();
    }
}

impl digest::Reset for GpuSha256 {
    fn reset(&mut self) {
        self.bytes.clear();
    }
}

impl digest::HashMarker for GpuSha256 {}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use sha2::{Digest as _, Sha256};
    use std::hash::{BuildHasher as _, Hasher as StdHasher};
    use twox_hash::{XxHash3_64, XxHash3_128, XxHash32, XxHash64};

    fn sample_messages() -> Vec<Vec<u8>> {
        let mut messages = vec![
            Vec::new(),
            b"a".to_vec(),
            b"abc".to_vec(),
            b"message digest".to_vec(),
            b"abcdefghijklmnopqrstuvwxyz".to_vec(),
            b"some bytes".to_vec(),
        ];

        for len in [
            1usize, 2, 3, 4, 5, 8, 9, 16, 17, 31, 32, 33, 55, 56, 57, 63, 64, 65, 95, 96, 97, 127,
            128, 129, 239, 240, 241, 1024, 1025, 4099,
        ] {
            messages.push(
                (0..len)
                    .map(|i| (i.wrapping_mul(37) & 0xff) as u8)
                    .collect(),
            );
        }

        messages
    }

    fn fuzz_messages() -> impl Strategy<Value = Vec<Vec<u8>>> {
        prop::collection::vec(prop::collection::vec(any::<u8>(), 0..=4096), 1..=24)
    }

    fn fuzz_secret() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(any::<u8>(), 136..=260)
    }

    fn custom_secret(len: usize) -> Vec<u8> {
        (0..len)
            .map(|i| {
                (i as u8)
                    .wrapping_mul(17)
                    .wrapping_add(91)
                    .rotate_left((i & 7) as u32)
            })
            .collect()
    }

    fn write_chunks(mut write: impl FnMut(&[u8]), input: &[u8]) {
        let mut offset = 0usize;
        for chunk_len in [1usize, 7, 3, 64, 2, 19, 251].into_iter().cycle() {
            if offset >= input.len() {
                break;
            }
            let end = offset.saturating_add(chunk_len).min(input.len());
            write(&input[offset..end]);
            offset = end;
        }
    }

    #[test]
    fn xxhash32_matches_twox_hash() -> Result<()> {
        let gpu = GpuHash::new()?;
        let messages = sample_messages();

        for seed in [0, 1, 1234, u32::MAX, 0xdead_cafe] {
            let got = gpu.xxhash32(seed, &messages)?;
            let expected: Vec<_> = messages
                .iter()
                .map(|message| XxHash32::oneshot(seed, message))
                .collect();
            assert_eq!(got, expected, "seed {seed:#x}");
        }

        Ok(())
    }

    #[test]
    fn xxhash3_64_matches_twox_hash() -> Result<()> {
        let gpu = GpuHash::new()?;
        let messages = sample_messages();

        let got = gpu.xxhash3_64(&messages)?;
        let expected: Vec<_> = messages
            .iter()
            .map(|message| XxHash3_64::oneshot(message))
            .collect();
        assert_eq!(got, expected);

        for seed in [1, 1234, u64::MAX, 0xdead_cafe_beef_f00d] {
            let got = gpu.xxhash3_64_with_seed(seed, &messages)?;
            let expected: Vec<_> = messages
                .iter()
                .map(|message| XxHash3_64::oneshot_with_seed(seed, message))
                .collect();
            assert_eq!(got, expected, "seed {seed:#x}");
        }

        Ok(())
    }

    #[test]
    fn xxhash3_64_with_secret_matches_twox_hash() -> Result<()> {
        let gpu = GpuHash::new()?;
        let messages = sample_messages();

        for secret_len in [136, 192, 257] {
            let secret = custom_secret(secret_len);
            let got = gpu.xxhash3_64_with_secret(&secret, &messages)?;
            let expected: Vec<_> = messages
                .iter()
                .map(|message| XxHash3_64::oneshot_with_secret(&secret, message).unwrap())
                .collect();
            assert_eq!(got, expected, "secret_len {secret_len}");
        }

        assert_eq!(
            gpu.xxhash3_64_with_secret(b"too short", &messages),
            Err(GpuHashError::SecretTooShort {
                minimum: 136,
                actual: 9,
            })
        );

        Ok(())
    }

    #[test]
    fn xxhash3_64_with_seed_and_secret_matches_twox_hash() -> Result<()> {
        let gpu = GpuHash::new()?;
        let messages = sample_messages();

        for seed in [0, 1, 1234, u64::MAX, 0xdead_cafe_beef_f00d] {
            for secret_len in [136, 192, 257] {
                let secret = custom_secret(secret_len);
                let got = gpu.xxhash3_64_with_seed_and_secret(seed, &secret, &messages)?;
                let expected: Vec<_> = messages
                    .iter()
                    .map(|message| {
                        XxHash3_64::oneshot_with_seed_and_secret(seed, &secret, message).unwrap()
                    })
                    .collect();
                assert_eq!(got, expected, "seed {seed:#x}, secret_len {secret_len}");
            }
        }

        Ok(())
    }

    #[test]
    fn xxhash3_128_matches_twox_hash() -> Result<()> {
        let gpu = GpuHash::new()?;
        let messages = sample_messages();

        let got = gpu.xxhash3_128(&messages)?;
        let expected: Vec<_> = messages
            .iter()
            .map(|message| XxHash3_128::oneshot(message))
            .collect();
        assert_eq!(got, expected);

        for seed in [1, 1234, u64::MAX, 0xdead_cafe_beef_f00d] {
            let got = gpu.xxhash3_128_with_seed(seed, &messages)?;
            let expected: Vec<_> = messages
                .iter()
                .map(|message| XxHash3_128::oneshot_with_seed(seed, message))
                .collect();
            assert_eq!(got, expected, "seed {seed:#x}");
        }

        Ok(())
    }

    #[test]
    fn xxhash3_128_with_secret_matches_twox_hash() -> Result<()> {
        let gpu = GpuHash::new()?;
        let messages = sample_messages();

        for secret_len in [136, 192, 257] {
            let secret = custom_secret(secret_len);
            let got = gpu.xxhash3_128_with_secret(&secret, &messages)?;
            let expected: Vec<_> = messages
                .iter()
                .map(|message| XxHash3_128::oneshot_with_secret(&secret, message).unwrap())
                .collect();
            assert_eq!(got, expected, "secret_len {secret_len}");
        }

        assert_eq!(
            gpu.xxhash3_128_with_secret(b"too short", &messages),
            Err(GpuHashError::SecretTooShort {
                minimum: 136,
                actual: 9,
            })
        );

        Ok(())
    }

    #[test]
    fn xxhash3_128_with_seed_and_secret_matches_twox_hash() -> Result<()> {
        let gpu = GpuHash::new()?;
        let messages = sample_messages();

        for seed in [0, 1, 1234, u64::MAX, 0xdead_cafe_beef_f00d] {
            for secret_len in [136, 192, 257] {
                let secret = custom_secret(secret_len);
                let got = gpu.xxhash3_128_with_seed_and_secret(seed, &secret, &messages)?;
                let expected: Vec<_> = messages
                    .iter()
                    .map(|message| {
                        XxHash3_128::oneshot_with_seed_and_secret(seed, &secret, message).unwrap()
                    })
                    .collect();
                assert_eq!(got, expected, "seed {seed:#x}, secret_len {secret_len}");
            }
        }

        Ok(())
    }

    #[test]
    fn xxhash64_matches_twox_hash() -> Result<()> {
        let gpu = GpuHash::new()?;
        let messages = sample_messages();

        for seed in [0, 1, 1234, u64::MAX, 0xdead_cafe_beef_f00d] {
            let got = gpu.xxhash64(seed, &messages)?;
            let expected: Vec<_> = messages
                .iter()
                .map(|message| XxHash64::oneshot(seed, message))
                .collect();
            assert_eq!(got, expected, "seed {seed:#x}");
        }

        assert_eq!(
            XxHash64::oneshot(1234, b"some bytes"),
            0xeab5_5659_a496_d78b
        );
        Ok(())
    }

    #[test]
    fn sha256_matches_rustcrypto() -> Result<()> {
        let gpu = GpuHash::new()?;
        let messages = sample_messages();
        let got = gpu.sha256(&messages)?;
        let expected: Vec<[u8; 32]> = messages
            .iter()
            .map(|message| Sha256::digest(message).into())
            .collect();

        assert_eq!(got, expected);
        Ok(())
    }

    #[test]
    fn prepared_batches_can_be_reused() -> Result<()> {
        let gpu = GpuHash::new()?;
        let messages = sample_messages();
        let batch = gpu.prepare_batch(&messages)?;

        let first32 = gpu.xxhash32_prepared(42, &batch)?;
        let second32 = gpu.xxhash32_prepared(42, &batch)?;
        assert_eq!(first32, second32);

        let first = gpu.xxhash64_prepared(42, &batch)?;
        let second = gpu.xxhash64_prepared(42, &batch)?;
        assert_eq!(first, second);

        let first3_64 = gpu.xxhash3_64_prepared(&batch)?;
        let second3_64 = gpu.xxhash3_64_prepared(&batch)?;
        assert_eq!(first3_64, second3_64);

        let first3_128 = gpu.xxhash3_128_prepared(&batch)?;
        let second3_128 = gpu.xxhash3_128_prepared(&batch)?;
        assert_eq!(first3_128, second3_128);

        let sha_first = gpu.sha256_prepared(&batch)?;
        let sha_second = gpu.sha256_prepared(&batch)?;
        assert_eq!(sha_first, sha_second);
        Ok(())
    }

    #[test]
    fn buffered_hashers_match_streaming_references() -> Result<()> {
        for message in sample_messages() {
            let mut expected32 = XxHash32::with_seed(0xdead_cafe);
            let mut got32 = GpuXxHash32::with_seed(0xdead_cafe)?;
            write_chunks(
                |chunk| {
                    StdHasher::write(&mut expected32, chunk);
                    got32.write(chunk);
                },
                &message,
            );
            assert_eq!(got32.finish_32()?, expected32.finish_32());
            assert_eq!(StdHasher::finish(&got32), StdHasher::finish(&expected32));
            assert_eq!(got32.total_len(), message.len() as u64);

            let mut expected64 = XxHash64::with_seed(0xdead_cafe_beef_f00d);
            let mut got64 = GpuXxHash64::with_seed(0xdead_cafe_beef_f00d)?;
            write_chunks(
                |chunk| {
                    StdHasher::write(&mut expected64, chunk);
                    got64.write(chunk);
                },
                &message,
            );
            assert_eq!(got64.finish_64()?, StdHasher::finish(&expected64));
            assert_eq!(StdHasher::finish(&got64), StdHasher::finish(&expected64));
            assert_eq!(got64.total_len(), message.len() as u64);

            let mut expected3_64 = XxHash3_64::with_seed(0x1234_5678_9abc_def0);
            let mut got3_64 = GpuXxHash3_64::with_seed(0x1234_5678_9abc_def0)?;
            write_chunks(
                |chunk| {
                    StdHasher::write(&mut expected3_64, chunk);
                    got3_64.write(chunk);
                },
                &message,
            );
            assert_eq!(got3_64.finish_64()?, StdHasher::finish(&expected3_64));
            assert_eq!(
                StdHasher::finish(&got3_64),
                StdHasher::finish(&expected3_64)
            );
            assert_eq!(got3_64.total_len(), message.len() as u64);

            let secret = custom_secret(257);
            let mut expected3_128 =
                XxHash3_128::with_seed_and_secret(0x1234_5678_9abc_def0, secret.clone()).unwrap();
            let mut got3_128 = GpuXxHash3_128::with_seed_and_secret(0x1234_5678_9abc_def0, secret)?;
            write_chunks(
                |chunk| {
                    expected3_128.write(chunk);
                    got3_128.write(chunk);
                },
                &message,
            );
            assert_eq!(got3_128.finish_128()?, expected3_128.finish_128());
            assert_eq!(got3_128.total_len(), message.len() as u64);

            let mut expected_sha = Sha256::new();
            let mut got_sha = GpuSha256::try_new()?;
            write_chunks(
                |chunk| {
                    expected_sha.update(chunk);
                    got_sha.write(chunk);
                },
                &message,
            );
            let expected_digest: [u8; 32] = expected_sha.finalize().into();
            assert_eq!(got_sha.try_finalize()?, expected_digest);
            assert_eq!(got_sha.total_len(), message.len() as u64);
        }

        Ok(())
    }

    #[test]
    fn build_hashers_match_streaming_references() {
        let message = b"builder-based streaming hash";

        let xx32_builder = GpuXxHash32State::with_seed(0xdead_cafe);
        assert_eq!(xx32_builder.seed(), 0xdead_cafe);
        let mut got32 = xx32_builder.build_hasher();
        let mut expected32 = XxHash32::with_seed(0xdead_cafe);
        got32.write(message);
        StdHasher::write(&mut expected32, message);
        assert_eq!(got32.finish_32().unwrap(), expected32.finish_32());
        assert_eq!(StdHasher::finish(&got32), StdHasher::finish(&expected32));

        let xx64_builder = GpuXxHash64State::with_seed(0xdead_cafe_beef_f00d);
        assert_eq!(xx64_builder.seed(), 0xdead_cafe_beef_f00d);
        let mut got64 = xx64_builder.build_hasher();
        let mut expected64 = XxHash64::with_seed(0xdead_cafe_beef_f00d);
        got64.write(message);
        StdHasher::write(&mut expected64, message);
        assert_eq!(got64.finish_64().unwrap(), StdHasher::finish(&expected64));
        assert_eq!(StdHasher::finish(&got64), StdHasher::finish(&expected64));

        let xxh3_builder = GpuXxHash3_64State::with_seed(0x1234_5678_9abc_def0);
        assert_eq!(xxh3_builder.seed(), 0x1234_5678_9abc_def0);
        let mut got3 = xxh3_builder.build_hasher();
        let mut expected3 = XxHash3_64::with_seed(0x1234_5678_9abc_def0);
        got3.write(message);
        StdHasher::write(&mut expected3, message);
        assert_eq!(got3.finish_64().unwrap(), StdHasher::finish(&expected3));
        assert_eq!(StdHasher::finish(&got3), StdHasher::finish(&expected3));
    }

    #[test]
    fn gpu_sha256_implements_digest_traits() {
        let mut hasher = GpuSha256::new();
        hasher.update(b"abc");
        let got: [u8; 32] = hasher.finalize().into();
        let expected: [u8; 32] = Sha256::digest(b"abc").into();
        assert_eq!(got, expected);

        let got: [u8; 32] = GpuSha256::digest(b"message digest").into();
        let expected: [u8; 32] = Sha256::digest(b"message digest").into();
        assert_eq!(got, expected);

        let mut hasher = GpuSha256::new();
        hasher.update(b"first");
        let first: [u8; 32] = hasher.finalize_reset().into();
        let expected_first: [u8; 32] = Sha256::digest(b"first").into();
        assert_eq!(first, expected_first);

        hasher.update(b"second");
        let second: [u8; 32] = hasher.finalize().into();
        let expected_second: [u8; 32] = Sha256::digest(b"second").into();
        assert_eq!(second, expected_second);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(96))]

        #[test]
        fn fuzz_xxhash32_matches_twox_hash(seed in any::<u32>(), messages in fuzz_messages()) {
            let gpu = GpuHash::new().unwrap();
            let got = gpu.xxhash32(seed, &messages).unwrap();
            let expected: Vec<_> = messages
                .iter()
                .map(|message| XxHash32::oneshot(seed, message))
                .collect();
            prop_assert_eq!(got, expected);
        }

        #[test]
        fn fuzz_xxhash64_matches_twox_hash(seed in any::<u64>(), messages in fuzz_messages()) {
            let gpu = GpuHash::new().unwrap();
            let got = gpu.xxhash64(seed, &messages).unwrap();
            let expected: Vec<_> = messages
                .iter()
                .map(|message| XxHash64::oneshot(seed, message))
                .collect();
            prop_assert_eq!(got, expected);
        }

        #[test]
        fn fuzz_xxhash3_64_matches_twox_hash(seed in any::<u64>(), messages in fuzz_messages()) {
            let gpu = GpuHash::new().unwrap();
            let got = gpu.xxhash3_64_with_seed(seed, &messages).unwrap();
            let expected: Vec<_> = messages
                .iter()
                .map(|message| XxHash3_64::oneshot_with_seed(seed, message))
                .collect();
            prop_assert_eq!(got, expected);
        }

        #[test]
        fn fuzz_xxhash3_128_matches_twox_hash(seed in any::<u64>(), messages in fuzz_messages()) {
            let gpu = GpuHash::new().unwrap();
            let got = gpu.xxhash3_128_with_seed(seed, &messages).unwrap();
            let expected: Vec<_> = messages
                .iter()
                .map(|message| XxHash3_128::oneshot_with_seed(seed, message))
                .collect();
            prop_assert_eq!(got, expected);
        }

        #[test]
        fn fuzz_xxhash3_64_with_secret_matches_twox_hash(secret in fuzz_secret(), messages in fuzz_messages()) {
            let gpu = GpuHash::new().unwrap();
            let got = gpu.xxhash3_64_with_secret(&secret, &messages).unwrap();
            let expected: Vec<_> = messages
                .iter()
                .map(|message| XxHash3_64::oneshot_with_secret(&secret, message).unwrap())
                .collect();
            prop_assert_eq!(got, expected);
        }

        #[test]
        fn fuzz_xxhash3_128_with_secret_matches_twox_hash(secret in fuzz_secret(), messages in fuzz_messages()) {
            let gpu = GpuHash::new().unwrap();
            let got = gpu.xxhash3_128_with_secret(&secret, &messages).unwrap();
            let expected: Vec<_> = messages
                .iter()
                .map(|message| XxHash3_128::oneshot_with_secret(&secret, message).unwrap())
                .collect();
            prop_assert_eq!(got, expected);
        }

        #[test]
        fn fuzz_xxhash3_64_with_seed_and_secret_matches_twox_hash(seed in any::<u64>(), secret in fuzz_secret(), messages in fuzz_messages()) {
            let gpu = GpuHash::new().unwrap();
            let got = gpu.xxhash3_64_with_seed_and_secret(seed, &secret, &messages).unwrap();
            let expected: Vec<_> = messages
                .iter()
                .map(|message| XxHash3_64::oneshot_with_seed_and_secret(seed, &secret, message).unwrap())
                .collect();
            prop_assert_eq!(got, expected);
        }

        #[test]
        fn fuzz_xxhash3_128_with_seed_and_secret_matches_twox_hash(seed in any::<u64>(), secret in fuzz_secret(), messages in fuzz_messages()) {
            let gpu = GpuHash::new().unwrap();
            let got = gpu.xxhash3_128_with_seed_and_secret(seed, &secret, &messages).unwrap();
            let expected: Vec<_> = messages
                .iter()
                .map(|message| XxHash3_128::oneshot_with_seed_and_secret(seed, &secret, message).unwrap())
                .collect();
            prop_assert_eq!(got, expected);
        }

        #[test]
        fn fuzz_sha256_matches_rustcrypto(messages in fuzz_messages()) {
            let gpu = GpuHash::new().unwrap();
            let got = gpu.sha256(&messages).unwrap();
            let expected: Vec<[u8; 32]> = messages
                .iter()
                .map(|message| Sha256::digest(message).into())
                .collect();
            prop_assert_eq!(got, expected);
        }
    }
}
