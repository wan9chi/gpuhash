use crate::{GpuHashError, Result};
use metal::{
    Buffer, CommandQueue, CompileOptions, ComputePipelineState, Device, MTLCommandBufferStatus,
    MTLResourceOptions, MTLSize, NSUInteger,
};
use objc::rc::autoreleasepool;
use std::{mem, ptr};

const SHADERS: &str = include_str!("shaders.metal");
const MESSAGE_ALIGN: usize = 8;
const METAL_SET_BYTES_LIMIT: usize = 4096;
const XXH3_SECRET_MINIMUM_LENGTH: usize = 136;
const XXH3_SECRET_DERIVED_FOR_LARGE: u32 = 0;
const XXH3_SECRET_CUSTOM_FOR_ALL: u32 = 1;
const XXH3_SECRET_CUSTOM_FOR_LARGE: u32 = 2;
const XXH3_DEFAULT_SECRET: [u8; 192] = [
    0xb8, 0xfe, 0x6c, 0x39, 0x23, 0xa4, 0x4b, 0xbe, 0x7c, 0x01, 0x81, 0x2c, 0xf7, 0x21, 0xad, 0x1c,
    0xde, 0xd4, 0x6d, 0xe9, 0x83, 0x90, 0x97, 0xdb, 0x72, 0x40, 0xa4, 0xa4, 0xb7, 0xb3, 0x67, 0x1f,
    0xcb, 0x79, 0xe6, 0x4e, 0xcc, 0xc0, 0xe5, 0x78, 0x82, 0x5a, 0xd0, 0x7d, 0xcc, 0xff, 0x72, 0x21,
    0xb8, 0x08, 0x46, 0x74, 0xf7, 0x43, 0x24, 0x8e, 0xe0, 0x35, 0x90, 0xe6, 0x81, 0x3a, 0x26, 0x4c,
    0x3c, 0x28, 0x52, 0xbb, 0x91, 0xc3, 0x00, 0xcb, 0x88, 0xd0, 0x65, 0x8b, 0x1b, 0x53, 0x2e, 0xa3,
    0x71, 0x64, 0x48, 0x97, 0xa2, 0x0d, 0xf9, 0x4e, 0x38, 0x19, 0xef, 0x46, 0xa9, 0xde, 0xac, 0xd8,
    0xa8, 0xfa, 0x76, 0x3f, 0xe3, 0x9c, 0x34, 0x3f, 0xf9, 0xdc, 0xbb, 0xc7, 0xc7, 0x0b, 0x4f, 0x1d,
    0x8a, 0x51, 0xe0, 0x4b, 0xcd, 0xb4, 0x59, 0x31, 0xc8, 0x9f, 0x7e, 0xc9, 0xd9, 0x78, 0x73, 0x64,
    0xea, 0xc5, 0xac, 0x83, 0x34, 0xd3, 0xeb, 0xc3, 0xc5, 0x81, 0xa0, 0xff, 0xfa, 0x13, 0x63, 0xeb,
    0x17, 0x0d, 0xdd, 0x51, 0xb7, 0xf0, 0xda, 0x49, 0xd3, 0x16, 0x55, 0x26, 0x29, 0xd4, 0x68, 0x9e,
    0x2b, 0x16, 0xbe, 0x58, 0x7d, 0x47, 0xa1, 0xfc, 0x8f, 0xf8, 0xb8, 0xd1, 0x7a, 0xd0, 0x31, 0xce,
    0x45, 0xcb, 0x3a, 0x8f, 0x95, 0x16, 0x04, 0x28, 0xaf, 0xd7, 0xfb, 0xca, 0xbb, 0x4b, 0x40, 0x7e,
];

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct MessageDesc {
    offset: u64,
    len: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct XxHash32Config {
    seed: u32,
    count: u32,
    _pad: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct XxHash64Config {
    seed: u64,
    count: u32,
    _pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct XxHash3Config {
    seed: u64,
    count: u32,
    secret_mode: u32,
    secret_len: u32,
    _pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Sha256Config {
    count: u32,
    _pad: u32,
    len: u64,
    stride: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct UniformXxHash32Config {
    seed: u32,
    count: u32,
    len: u64,
    stride: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct UniformXxHash64Config {
    seed: u64,
    count: u32,
    _pad: u32,
    len: u64,
    stride: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct UniformXxHash3Config {
    seed: u64,
    count: u32,
    secret_mode: u32,
    secret_len: u32,
    _pad: u32,
    len: u64,
    stride: u64,
}

#[derive(Clone, Copy, Debug)]
struct UniformLayout {
    len: usize,
    stride: usize,
}

pub struct PreparedBatch {
    input: Buffer,
    descs: Buffer,
    count: usize,
    total_bytes: usize,
    uniform: Option<UniformLayout>,
}

impl PreparedBatch {
    pub fn count(&self) -> usize {
        self.count
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }
}

pub struct GpuHash {
    device: Device,
    queue: CommandQueue,
    xxhash32_pipeline: ComputePipelineState,
    xxhash32_uniform_pipeline: ComputePipelineState,
    xxhash64_pipeline: ComputePipelineState,
    xxhash64_uniform_pipeline: ComputePipelineState,
    xxhash3_64_pipeline: ComputePipelineState,
    xxhash3_64_uniform_pipeline: ComputePipelineState,
    xxhash3_128_pipeline: ComputePipelineState,
    xxhash3_128_uniform_pipeline: ComputePipelineState,
    sha256_pipeline: ComputePipelineState,
    sha256_uniform_pipeline: ComputePipelineState,
}

impl GpuHash {
    pub fn new() -> Result<Self> {
        autoreleasepool(|| {
            let device = Device::system_default().ok_or(GpuHashError::MetalUnavailable)?;
            let queue = device.new_command_queue();

            let options = CompileOptions::new();
            options.set_fast_math_enabled(false);
            let library = device
                .new_library_with_source(SHADERS, &options)
                .map_err(GpuHashError::Metal)?;

            let xxhash32_function = library
                .get_function("xxhash32_batch", None)
                .map_err(GpuHashError::Metal)?;
            let xxhash32_uniform_function = library
                .get_function("xxhash32_uniform_batch", None)
                .map_err(GpuHashError::Metal)?;
            let xxhash64_function = library
                .get_function("xxhash64_batch", None)
                .map_err(GpuHashError::Metal)?;
            let xxhash64_uniform_function = library
                .get_function("xxhash64_uniform_batch", None)
                .map_err(GpuHashError::Metal)?;
            let xxhash3_64_function = library
                .get_function("xxhash3_64_batch", None)
                .map_err(GpuHashError::Metal)?;
            let xxhash3_64_uniform_function = library
                .get_function("xxhash3_64_uniform_batch", None)
                .map_err(GpuHashError::Metal)?;
            let xxhash3_128_function = library
                .get_function("xxhash3_128_batch", None)
                .map_err(GpuHashError::Metal)?;
            let xxhash3_128_uniform_function = library
                .get_function("xxhash3_128_uniform_batch", None)
                .map_err(GpuHashError::Metal)?;
            let sha256_function = library
                .get_function("sha256_batch", None)
                .map_err(GpuHashError::Metal)?;
            let sha256_uniform_function = library
                .get_function("sha256_uniform_batch", None)
                .map_err(GpuHashError::Metal)?;

            let xxhash32_pipeline = device
                .new_compute_pipeline_state_with_function(&xxhash32_function)
                .map_err(|err| GpuHashError::Metal(format!("{err:?}")))?;
            let xxhash32_uniform_pipeline = device
                .new_compute_pipeline_state_with_function(&xxhash32_uniform_function)
                .map_err(|err| GpuHashError::Metal(format!("{err:?}")))?;
            let xxhash64_pipeline = device
                .new_compute_pipeline_state_with_function(&xxhash64_function)
                .map_err(|err| GpuHashError::Metal(format!("{err:?}")))?;
            let xxhash64_uniform_pipeline = device
                .new_compute_pipeline_state_with_function(&xxhash64_uniform_function)
                .map_err(|err| GpuHashError::Metal(format!("{err:?}")))?;
            let xxhash3_64_pipeline = device
                .new_compute_pipeline_state_with_function(&xxhash3_64_function)
                .map_err(|err| GpuHashError::Metal(format!("{err:?}")))?;
            let xxhash3_64_uniform_pipeline = device
                .new_compute_pipeline_state_with_function(&xxhash3_64_uniform_function)
                .map_err(|err| GpuHashError::Metal(format!("{err:?}")))?;
            let xxhash3_128_pipeline = device
                .new_compute_pipeline_state_with_function(&xxhash3_128_function)
                .map_err(|err| GpuHashError::Metal(format!("{err:?}")))?;
            let xxhash3_128_uniform_pipeline = device
                .new_compute_pipeline_state_with_function(&xxhash3_128_uniform_function)
                .map_err(|err| GpuHashError::Metal(format!("{err:?}")))?;
            let sha256_pipeline = device
                .new_compute_pipeline_state_with_function(&sha256_function)
                .map_err(|err| GpuHashError::Metal(format!("{err:?}")))?;
            let sha256_uniform_pipeline = device
                .new_compute_pipeline_state_with_function(&sha256_uniform_function)
                .map_err(|err| GpuHashError::Metal(format!("{err:?}")))?;

            Ok(Self {
                device,
                queue,
                xxhash32_pipeline,
                xxhash32_uniform_pipeline,
                xxhash64_pipeline,
                xxhash64_uniform_pipeline,
                xxhash3_64_pipeline,
                xxhash3_64_uniform_pipeline,
                xxhash3_128_pipeline,
                xxhash3_128_uniform_pipeline,
                sha256_pipeline,
                sha256_uniform_pipeline,
            })
        })
    }

    pub fn prepare_batch<M: AsRef<[u8]>>(&self, messages: &[M]) -> Result<PreparedBatch> {
        let count = messages.len();
        if count > u32::MAX as usize {
            return Err(GpuHashError::InputTooLarge);
        }

        let mut total_bytes = 0usize;
        let mut padded_bytes = 0usize;
        let mut uniform_len = messages.first().map(|message| message.as_ref().len());
        for message in messages {
            let len = message.as_ref().len();
            if uniform_len.is_some_and(|first_len| first_len != len) {
                uniform_len = None;
            }
            total_bytes = total_bytes
                .checked_add(len)
                .ok_or(GpuHashError::InputTooLarge)?;
            padded_bytes = align_up(padded_bytes, MESSAGE_ALIGN)
                .checked_add(len)
                .ok_or(GpuHashError::InputTooLarge)?;
        }

        let desc_bytes = count
            .checked_mul(mem::size_of::<MessageDesc>())
            .ok_or(GpuHashError::InputTooLarge)?;

        let input = self.device.new_buffer(
            padded_bytes.max(1) as u64,
            MTLResourceOptions::StorageModeShared,
        );
        let descs = self.device.new_buffer(
            desc_bytes.max(1) as u64,
            MTLResourceOptions::StorageModeShared,
        );

        unsafe {
            let mut offset = 0usize;
            let desc_ptr = descs.contents().cast::<MessageDesc>();
            let input_ptr = input.contents().cast::<u8>();

            for (idx, message) in messages.iter().enumerate() {
                let bytes = message.as_ref();
                offset = align_up(offset, MESSAGE_ALIGN);
                *desc_ptr.add(idx) = MessageDesc {
                    offset: offset as u64,
                    len: bytes.len() as u64,
                };

                if !bytes.is_empty() {
                    ptr::copy_nonoverlapping(bytes.as_ptr(), input_ptr.add(offset), bytes.len());
                }
                offset += bytes.len();
            }
        }

        Ok(PreparedBatch {
            input,
            descs,
            count,
            total_bytes,
            uniform: uniform_len.map(|len| UniformLayout {
                len,
                stride: align_up(len, MESSAGE_ALIGN),
            }),
        })
    }

    pub fn xxhash32(&self, seed: u32, batch: &PreparedBatch) -> Result<Vec<u32>> {
        let mut output = Vec::with_capacity(batch.count);
        if batch.count == 0 {
            return Ok(output);
        }

        let out_buffer = self.output_buffer(&mut output, batch.count);
        if let Some(uniform) = batch.uniform {
            let config = UniformXxHash32Config {
                seed,
                count: batch.count as u32,
                len: uniform.len as u64,
                stride: uniform.stride as u64,
            };
            self.dispatch(&self.xxhash32_uniform_pipeline, batch.count, |encoder| {
                encoder.set_buffer(0, Some(&batch.input), 0);
                encoder.set_buffer(1, Some(&out_buffer), 0);
                Self::set_config_bytes(encoder, 2, &config);
            })?;
        } else {
            let config = XxHash32Config {
                seed,
                count: batch.count as u32,
                _pad: [0; 2],
            };
            self.dispatch(&self.xxhash32_pipeline, batch.count, |encoder| {
                encoder.set_buffer(0, Some(&batch.input), 0);
                encoder.set_buffer(1, Some(&batch.descs), 0);
                encoder.set_buffer(2, Some(&out_buffer), 0);
                Self::set_config_bytes(encoder, 3, &config);
            })?;
        }

        unsafe {
            output.set_len(batch.count);
        }
        Ok(output)
    }

    pub fn xxhash64(&self, seed: u64, batch: &PreparedBatch) -> Result<Vec<u64>> {
        let mut output = Vec::with_capacity(batch.count);
        if batch.count == 0 {
            return Ok(output);
        }

        let out_buffer = self.output_buffer(&mut output, batch.count);
        let config = XxHash64Config {
            seed,
            count: batch.count as u32,
            _pad: 0,
        };
        if let Some(uniform) = batch.uniform {
            let config = UniformXxHash64Config {
                seed,
                count: batch.count as u32,
                _pad: 0,
                len: uniform.len as u64,
                stride: uniform.stride as u64,
            };
            self.dispatch(&self.xxhash64_uniform_pipeline, batch.count, |encoder| {
                encoder.set_buffer(0, Some(&batch.input), 0);
                encoder.set_buffer(1, Some(&out_buffer), 0);
                Self::set_config_bytes(encoder, 2, &config);
            })?;
        } else {
            self.dispatch(&self.xxhash64_pipeline, batch.count, |encoder| {
                encoder.set_buffer(0, Some(&batch.input), 0);
                encoder.set_buffer(1, Some(&batch.descs), 0);
                encoder.set_buffer(2, Some(&out_buffer), 0);
                Self::set_config_bytes(encoder, 3, &config);
            })?;
        }

        unsafe {
            output.set_len(batch.count);
        }
        Ok(output)
    }

    pub fn xxhash3_64(&self, seed: u64, batch: &PreparedBatch) -> Result<Vec<u64>> {
        let secret = derive_xxh3_secret(seed);
        self.xxhash3_64_with_secret_mode(seed, batch, &secret, XXH3_SECRET_DERIVED_FOR_LARGE)
    }

    pub fn xxhash3_64_with_secret(&self, secret: &[u8], batch: &PreparedBatch) -> Result<Vec<u64>> {
        validate_xxh3_secret(secret)?;
        self.xxhash3_64_with_secret_mode(0, batch, secret, XXH3_SECRET_CUSTOM_FOR_ALL)
    }

    pub fn xxhash3_64_with_seed_and_secret(
        &self,
        seed: u64,
        secret: &[u8],
        batch: &PreparedBatch,
    ) -> Result<Vec<u64>> {
        validate_xxh3_secret(secret)?;
        self.xxhash3_64_with_secret_mode(seed, batch, secret, XXH3_SECRET_CUSTOM_FOR_LARGE)
    }

    fn xxhash3_64_with_secret_mode(
        &self,
        seed: u64,
        batch: &PreparedBatch,
        secret: &[u8],
        secret_mode: u32,
    ) -> Result<Vec<u64>> {
        let mut output = Vec::with_capacity(batch.count);
        if batch.count == 0 {
            return Ok(output);
        }
        if secret.len() > u32::MAX as usize {
            return Err(GpuHashError::InputTooLarge);
        }

        let out_buffer = self.output_buffer(&mut output, batch.count);
        let secret_buffer = self.secret_buffer_if_needed(secret);

        if let Some(uniform) = batch.uniform {
            let config = UniformXxHash3Config {
                seed,
                count: batch.count as u32,
                secret_mode,
                secret_len: secret.len() as u32,
                _pad: 0,
                len: uniform.len as u64,
                stride: uniform.stride as u64,
            };
            self.dispatch(&self.xxhash3_64_uniform_pipeline, batch.count, |encoder| {
                encoder.set_buffer(0, Some(&batch.input), 0);
                encoder.set_buffer(1, Some(&out_buffer), 0);
                Self::set_config_bytes(encoder, 2, &config);
                Self::set_secret_bytes(encoder, 3, secret, secret_buffer.as_ref());
            })?;
        } else {
            let config = XxHash3Config {
                seed,
                count: batch.count as u32,
                secret_mode,
                secret_len: secret.len() as u32,
                _pad: 0,
            };
            self.dispatch(&self.xxhash3_64_pipeline, batch.count, |encoder| {
                encoder.set_buffer(0, Some(&batch.input), 0);
                encoder.set_buffer(1, Some(&batch.descs), 0);
                encoder.set_buffer(2, Some(&out_buffer), 0);
                Self::set_config_bytes(encoder, 3, &config);
                Self::set_secret_bytes(encoder, 4, secret, secret_buffer.as_ref());
            })?;
        }

        unsafe {
            output.set_len(batch.count);
        }
        Ok(output)
    }

    pub fn xxhash3_128(&self, seed: u64, batch: &PreparedBatch) -> Result<Vec<u128>> {
        let secret = derive_xxh3_secret(seed);
        self.xxhash3_128_with_secret_mode(seed, batch, &secret, XXH3_SECRET_DERIVED_FOR_LARGE)
    }

    pub fn xxhash3_128_with_secret(
        &self,
        secret: &[u8],
        batch: &PreparedBatch,
    ) -> Result<Vec<u128>> {
        validate_xxh3_secret(secret)?;
        self.xxhash3_128_with_secret_mode(0, batch, secret, XXH3_SECRET_CUSTOM_FOR_ALL)
    }

    pub fn xxhash3_128_with_seed_and_secret(
        &self,
        seed: u64,
        secret: &[u8],
        batch: &PreparedBatch,
    ) -> Result<Vec<u128>> {
        validate_xxh3_secret(secret)?;
        self.xxhash3_128_with_secret_mode(seed, batch, secret, XXH3_SECRET_CUSTOM_FOR_LARGE)
    }

    fn xxhash3_128_with_secret_mode(
        &self,
        seed: u64,
        batch: &PreparedBatch,
        secret: &[u8],
        secret_mode: u32,
    ) -> Result<Vec<u128>> {
        let mut output = Vec::with_capacity(batch.count);
        if batch.count == 0 {
            return Ok(output);
        }
        if secret.len() > u32::MAX as usize {
            return Err(GpuHashError::InputTooLarge);
        }

        let out_buffer = self.output_buffer(&mut output, batch.count);
        let secret_buffer = self.secret_buffer_if_needed(secret);

        if let Some(uniform) = batch.uniform {
            let config = UniformXxHash3Config {
                seed,
                count: batch.count as u32,
                secret_mode,
                secret_len: secret.len() as u32,
                _pad: 0,
                len: uniform.len as u64,
                stride: uniform.stride as u64,
            };
            self.dispatch(&self.xxhash3_128_uniform_pipeline, batch.count, |encoder| {
                encoder.set_buffer(0, Some(&batch.input), 0);
                encoder.set_buffer(1, Some(&out_buffer), 0);
                Self::set_config_bytes(encoder, 2, &config);
                Self::set_secret_bytes(encoder, 3, secret, secret_buffer.as_ref());
            })?;
        } else {
            let config = XxHash3Config {
                seed,
                count: batch.count as u32,
                secret_mode,
                secret_len: secret.len() as u32,
                _pad: 0,
            };
            self.dispatch(&self.xxhash3_128_pipeline, batch.count, |encoder| {
                encoder.set_buffer(0, Some(&batch.input), 0);
                encoder.set_buffer(1, Some(&batch.descs), 0);
                encoder.set_buffer(2, Some(&out_buffer), 0);
                Self::set_config_bytes(encoder, 3, &config);
                Self::set_secret_bytes(encoder, 4, secret, secret_buffer.as_ref());
            })?;
        }

        unsafe {
            output.set_len(batch.count);
        }
        Ok(output)
    }

    pub fn sha256(&self, batch: &PreparedBatch) -> Result<Vec<[u8; 32]>> {
        let mut output = Vec::with_capacity(batch.count);
        if batch.count == 0 {
            return Ok(output);
        }

        let out_buffer = self.output_buffer(&mut output, batch.count);
        if let Some(uniform) = batch.uniform {
            let config = Sha256Config {
                count: batch.count as u32,
                _pad: 0,
                len: uniform.len as u64,
                stride: uniform.stride as u64,
            };
            self.dispatch(&self.sha256_uniform_pipeline, batch.count, |encoder| {
                encoder.set_buffer(0, Some(&batch.input), 0);
                encoder.set_buffer(1, Some(&out_buffer), 0);
                Self::set_config_bytes(encoder, 2, &config);
            })?;
        } else {
            let config = Sha256Config {
                count: batch.count as u32,
                _pad: 0,
                len: 0,
                stride: 0,
            };
            self.dispatch(&self.sha256_pipeline, batch.count, |encoder| {
                encoder.set_buffer(0, Some(&batch.input), 0);
                encoder.set_buffer(1, Some(&batch.descs), 0);
                encoder.set_buffer(2, Some(&out_buffer), 0);
                Self::set_config_bytes(encoder, 3, &config);
            })?;
        }

        unsafe {
            output.set_len(batch.count);
        }
        Ok(output)
    }

    fn output_buffer<T: Copy>(&self, output: &mut Vec<T>, len: usize) -> Buffer {
        debug_assert!(output.capacity() >= len);
        // The GPU writes every element before callers set the Vec length.
        self.device.new_buffer_with_bytes_no_copy(
            output.as_mut_ptr().cast(),
            (len * mem::size_of::<T>()) as u64,
            MTLResourceOptions::StorageModeShared,
            None,
        )
    }

    fn secret_buffer_if_needed(&self, secret: &[u8]) -> Option<Buffer> {
        (secret.len() > METAL_SET_BYTES_LIMIT).then(|| {
            self.device.new_buffer_with_data(
                secret.as_ptr().cast(),
                secret.len() as u64,
                MTLResourceOptions::StorageModeShared,
            )
        })
    }

    fn set_config_bytes<T>(
        encoder: &metal::ComputeCommandEncoderRef,
        index: NSUInteger,
        value: &T,
    ) {
        encoder.set_bytes(
            index,
            mem::size_of::<T>() as NSUInteger,
            (value as *const T).cast(),
        );
    }

    fn set_secret_bytes(
        encoder: &metal::ComputeCommandEncoderRef,
        index: NSUInteger,
        secret: &[u8],
        buffer: Option<&Buffer>,
    ) {
        if let Some(buffer) = buffer {
            encoder.set_buffer(index, Some(buffer), 0);
        } else {
            encoder.set_bytes(index, secret.len() as NSUInteger, secret.as_ptr().cast());
        }
    }

    fn dispatch(
        &self,
        pipeline: &ComputePipelineState,
        count: usize,
        bind: impl FnOnce(&metal::ComputeCommandEncoderRef),
    ) -> Result<()> {
        autoreleasepool(|| {
            let command_buffer = self.queue.new_command_buffer();
            let encoder = command_buffer.new_compute_command_encoder();
            encoder.set_compute_pipeline_state(pipeline);
            bind(encoder);

            let width = pipeline.thread_execution_width().max(1);
            let max_threads = pipeline.max_total_threads_per_threadgroup().max(width);
            let threads_per_group = width.min(max_threads);
            encoder.dispatch_threads(
                MTLSize {
                    width: count as NSUInteger,
                    height: 1,
                    depth: 1,
                },
                MTLSize {
                    width: threads_per_group,
                    height: 1,
                    depth: 1,
                },
            );
            encoder.end_encoding();

            command_buffer.commit();
            command_buffer.wait_until_completed();

            match command_buffer.status() {
                MTLCommandBufferStatus::Completed => Ok(()),
                status => Err(GpuHashError::Metal(format!(
                    "command buffer ended with status {status:?}"
                ))),
            }
        })
    }
}

fn align_up(value: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (value + align - 1) & !(align - 1)
}

fn validate_xxh3_secret(secret: &[u8]) -> Result<()> {
    if secret.len() < XXH3_SECRET_MINIMUM_LENGTH {
        return Err(GpuHashError::SecretTooShort {
            minimum: XXH3_SECRET_MINIMUM_LENGTH,
            actual: secret.len(),
        });
    }

    Ok(())
}

fn derive_xxh3_secret(seed: u64) -> [u8; 192] {
    let mut secret = XXH3_DEFAULT_SECRET;
    if seed == 0 {
        return secret;
    }

    for pair in secret.chunks_exact_mut(16) {
        let a = u64::from_le_bytes(pair[0..8].try_into().expect("exact chunk"));
        let b = u64::from_le_bytes(pair[8..16].try_into().expect("exact chunk"));
        pair[0..8].copy_from_slice(&a.wrapping_add(seed).to_le_bytes());
        pair[8..16].copy_from_slice(&b.wrapping_sub(seed).to_le_bytes());
    }

    secret
}
