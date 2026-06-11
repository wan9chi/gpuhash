#include <metal_stdlib>

using namespace metal;

struct MessageDesc {
    ulong offset;
    ulong len;
};

struct XxHash32Config {
    uint seed;
    uint count;
    uint2 _pad;
};

struct XxHash64Config {
    ulong seed;
    uint count;
    uint _pad;
};

struct XxHash3Config {
    ulong seed;
    uint count;
    uint _pad;
};

struct Sha256Config {
    uint count;
    uint3 _pad;
};

constant uint XXH32_PRIME1 = 0x9E3779B1U;
constant uint XXH32_PRIME2 = 0x85EBCA77U;
constant uint XXH32_PRIME3 = 0xC2B2AE3DU;
constant uint XXH32_PRIME4 = 0x27D4EB2FU;
constant uint XXH32_PRIME5 = 0x165667B1U;

constant ulong XXH64_PRIME1 = 11400714785074694791UL;
constant ulong XXH64_PRIME2 = 14029467366897019727UL;
constant ulong XXH64_PRIME3 =  1609587929392839161UL;
constant ulong XXH64_PRIME4 =  9650029242287828579UL;
constant ulong XXH64_PRIME5 =  2870177450012600261UL;
constant ulong XXH3_PRIME_MX1 = 0x165667919E3779F9UL;
constant ulong XXH3_PRIME_MX2 = 0x9FB21C651E98DF25UL;

constant uchar XXH3_DEFAULT_SECRET[192] = {
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
};

inline ulong rotl64(ulong value, uint amount) {
    return (value << amount) | (value >> (64 - amount));
}

inline uint rotl32(uint value, uint amount) {
    return (value << amount) | (value >> (32 - amount));
}

inline uint rotr32(uint value, uint amount) {
    return (value >> amount) | (value << (32 - amount));
}

inline ulong read_le64(device const uchar *ptr) {
    return *reinterpret_cast<device const ulong *>(ptr);
}

inline uint read_le32(device const uchar *ptr) {
    return *reinterpret_cast<device const uint *>(ptr);
}

inline uint read_be32(device const uchar *ptr) {
    return ((uint)ptr[0] << 24)
        | ((uint)ptr[1] << 16)
        | ((uint)ptr[2] << 8)
        | (uint)ptr[3];
}

inline uint xxh32_round(uint acc, uint input) {
    acc += input * XXH32_PRIME2;
    acc = rotl32(acc, 13);
    acc *= XXH32_PRIME1;
    return acc;
}

inline uint xxh32_avalanche(uint hash) {
    hash ^= hash >> 15;
    hash *= XXH32_PRIME2;
    hash ^= hash >> 13;
    hash *= XXH32_PRIME3;
    hash ^= hash >> 16;
    return hash;
}

inline uint xxhash32_one(device const uchar *data, ulong len, uint seed) {
    ulong index = 0;
    uint hash;

    if (len >= 16) {
        uint v1 = seed + XXH32_PRIME1 + XXH32_PRIME2;
        uint v2 = seed + XXH32_PRIME2;
        uint v3 = seed;
        uint v4 = seed - XXH32_PRIME1;

        ulong limit = len - 16;
        while (index <= limit) {
            v1 = xxh32_round(v1, read_le32(data + index));
            index += 4;
            v2 = xxh32_round(v2, read_le32(data + index));
            index += 4;
            v3 = xxh32_round(v3, read_le32(data + index));
            index += 4;
            v4 = xxh32_round(v4, read_le32(data + index));
            index += 4;
        }

        hash = rotl32(v1, 1) + rotl32(v2, 7) + rotl32(v3, 12) + rotl32(v4, 18);
    } else {
        hash = seed + XXH32_PRIME5;
    }

    hash += (uint)len;

    while (index + 4 <= len) {
        hash += read_le32(data + index) * XXH32_PRIME3;
        hash = rotl32(hash, 17) * XXH32_PRIME4;
        index += 4;
    }

    while (index < len) {
        hash += (uint)data[index] * XXH32_PRIME5;
        hash = rotl32(hash, 11) * XXH32_PRIME1;
        index += 1;
    }

    return xxh32_avalanche(hash);
}

kernel void xxhash32_batch(device const uchar *input [[buffer(0)]],
                           device const MessageDesc *descs [[buffer(1)]],
                           device uint *output [[buffer(2)]],
                           constant XxHash32Config &config [[buffer(3)]],
                           uint gid [[thread_position_in_grid]]) {
    if (gid >= config.count) {
        return;
    }

    MessageDesc desc = descs[gid];
    output[gid] = xxhash32_one(input + desc.offset, desc.len, config.seed);
}

inline ulong xxh64_round(ulong acc, ulong input) {
    acc += input * XXH64_PRIME2;
    acc = rotl64(acc, 31);
    acc *= XXH64_PRIME1;
    return acc;
}

inline ulong xxh64_merge_round(ulong acc, ulong val) {
    val = xxh64_round(0, val);
    acc ^= val;
    acc = acc * XXH64_PRIME1 + XXH64_PRIME4;
    return acc;
}

inline ulong xxh64_avalanche(ulong hash) {
    hash ^= hash >> 33;
    hash *= XXH64_PRIME2;
    hash ^= hash >> 29;
    hash *= XXH64_PRIME3;
    hash ^= hash >> 32;
    return hash;
}

inline ulong xxhash64_one(device const uchar *data, ulong len, ulong seed) {
    ulong index = 0;
    ulong hash;

    if (len >= 32) {
        ulong v1 = seed + XXH64_PRIME1 + XXH64_PRIME2;
        ulong v2 = seed + XXH64_PRIME2;
        ulong v3 = seed;
        ulong v4 = seed - XXH64_PRIME1;

        ulong limit = len - 32;
        while (index <= limit) {
            v1 = xxh64_round(v1, read_le64(data + index));
            index += 8;
            v2 = xxh64_round(v2, read_le64(data + index));
            index += 8;
            v3 = xxh64_round(v3, read_le64(data + index));
            index += 8;
            v4 = xxh64_round(v4, read_le64(data + index));
            index += 8;
        }

        hash = rotl64(v1, 1) + rotl64(v2, 7) + rotl64(v3, 12) + rotl64(v4, 18);
        hash = xxh64_merge_round(hash, v1);
        hash = xxh64_merge_round(hash, v2);
        hash = xxh64_merge_round(hash, v3);
        hash = xxh64_merge_round(hash, v4);
    } else {
        hash = seed + XXH64_PRIME5;
    }

    hash += len;

    while (index + 8 <= len) {
        ulong lane = xxh64_round(0, read_le64(data + index));
        hash ^= lane;
        hash = rotl64(hash, 27) * XXH64_PRIME1 + XXH64_PRIME4;
        index += 8;
    }

    if (index + 4 <= len) {
        hash ^= (ulong)read_le32(data + index) * XXH64_PRIME1;
        hash = rotl64(hash, 23) * XXH64_PRIME2 + XXH64_PRIME3;
        index += 4;
    }

    while (index < len) {
        hash ^= (ulong)data[index] * XXH64_PRIME5;
        hash = rotl64(hash, 11) * XXH64_PRIME1;
        index += 1;
    }

    return xxh64_avalanche(hash);
}

kernel void xxhash64_batch(device const uchar *input [[buffer(0)]],
                           device const MessageDesc *descs [[buffer(1)]],
                           device ulong *output [[buffer(2)]],
                           constant XxHash64Config &config [[buffer(3)]],
                           uint gid [[thread_position_in_grid]]) {
    if (gid >= config.count) {
        return;
    }

    MessageDesc desc = descs[gid];
    output[gid] = xxhash64_one(input + desc.offset, desc.len, config.seed);
}

struct U128Value {
    ulong low;
    ulong high;
};

inline U128Value make_u128(ulong low, ulong high) {
    U128Value value;
    value.low = low;
    value.high = high;
    return value;
}

inline uint bswap32(uint value) {
    return ((value & 0x000000ffU) << 24)
        | ((value & 0x0000ff00U) << 8)
        | ((value & 0x00ff0000U) >> 8)
        | ((value & 0xff000000U) >> 24);
}

inline ulong bswap64(ulong value) {
    return ((ulong)bswap32((uint)value) << 32) | (ulong)bswap32((uint)(value >> 32));
}

inline ulong read_le64_unaligned(device const uchar *ptr) {
    return (ulong)ptr[0]
        | ((ulong)ptr[1] << 8)
        | ((ulong)ptr[2] << 16)
        | ((ulong)ptr[3] << 24)
        | ((ulong)ptr[4] << 32)
        | ((ulong)ptr[5] << 40)
        | ((ulong)ptr[6] << 48)
        | ((ulong)ptr[7] << 56);
}

inline uint read_le32_unaligned(device const uchar *ptr) {
    return (uint)ptr[0]
        | ((uint)ptr[1] << 8)
        | ((uint)ptr[2] << 16)
        | ((uint)ptr[3] << 24);
}

inline ulong xxh3_default_secret_u64(ulong offset) {
    return (ulong)XXH3_DEFAULT_SECRET[offset]
        | ((ulong)XXH3_DEFAULT_SECRET[offset + 1] << 8)
        | ((ulong)XXH3_DEFAULT_SECRET[offset + 2] << 16)
        | ((ulong)XXH3_DEFAULT_SECRET[offset + 3] << 24)
        | ((ulong)XXH3_DEFAULT_SECRET[offset + 4] << 32)
        | ((ulong)XXH3_DEFAULT_SECRET[offset + 5] << 40)
        | ((ulong)XXH3_DEFAULT_SECRET[offset + 6] << 48)
        | ((ulong)XXH3_DEFAULT_SECRET[offset + 7] << 56);
}

inline uint xxh3_default_secret_u32(ulong offset) {
    return (uint)XXH3_DEFAULT_SECRET[offset]
        | ((uint)XXH3_DEFAULT_SECRET[offset + 1] << 8)
        | ((uint)XXH3_DEFAULT_SECRET[offset + 2] << 16)
        | ((uint)XXH3_DEFAULT_SECRET[offset + 3] << 24);
}

inline ulong xxh3_derived_secret_u64(device const uchar *secret, ulong offset) {
    return (ulong)secret[offset]
        | ((ulong)secret[offset + 1] << 8)
        | ((ulong)secret[offset + 2] << 16)
        | ((ulong)secret[offset + 3] << 24)
        | ((ulong)secret[offset + 4] << 32)
        | ((ulong)secret[offset + 5] << 40)
        | ((ulong)secret[offset + 6] << 48)
        | ((ulong)secret[offset + 7] << 56);
}

inline U128Value mul64_to128(ulong a, ulong b) {
    ulong mask = 0xffffffffUL;
    ulong a0 = a & mask;
    ulong a1 = a >> 32;
    ulong b0 = b & mask;
    ulong b1 = b >> 32;

    ulong p00 = a0 * b0;
    ulong p01 = a0 * b1;
    ulong p10 = a1 * b0;
    ulong p11 = a1 * b1;

    ulong middle = (p00 >> 32) + (p01 & mask) + (p10 & mask);
    ulong low = (p00 & mask) | (middle << 32);
    ulong high = p11 + (p01 >> 32) + (p10 >> 32) + (middle >> 32);
    return make_u128(low, high);
}

inline U128Value mul128_by64(ulong low, ulong high, ulong rhs) {
    U128Value lo_mul = mul64_to128(low, rhs);
    U128Value hi_mul = mul64_to128(high, rhs);
    return make_u128(lo_mul.low, lo_mul.high + hi_mul.low);
}

inline ulong xor_fold128(U128Value value) {
    return value.low ^ value.high;
}

inline ulong xxh3_avalanche(ulong value) {
    value ^= value >> 37;
    value *= XXH3_PRIME_MX1;
    value ^= value >> 32;
    return value;
}

inline uint xxh3_1_to_3_combined(device const uchar *data, ulong len) {
    return (uint)data[len - 1]
        | ((uint)len << 8)
        | ((uint)data[0] << 16)
        | ((uint)data[len >> 1] << 24);
}

inline ulong xxh3_mix_step_default(device const uchar *data, ulong secret_offset, ulong seed) {
    ulong data0 = read_le64_unaligned(data);
    ulong data1 = read_le64_unaligned(data + 8);
    ulong secret0 = xxh3_default_secret_u64(secret_offset);
    ulong secret1 = xxh3_default_secret_u64(secret_offset + 8);
    U128Value product = mul64_to128(data0 ^ (secret0 + seed), data1 ^ (secret1 - seed));
    return xor_fold128(product);
}

inline void xxh3_mix_two_chunks_default(thread ulong acc[2],
                                        device const uchar *data1,
                                        device const uchar *data2,
                                        ulong secret_offset,
                                        ulong seed) {
    ulong data10 = read_le64_unaligned(data1);
    ulong data11 = read_le64_unaligned(data1 + 8);
    ulong data20 = read_le64_unaligned(data2);
    ulong data21 = read_le64_unaligned(data2 + 8);

    acc[0] += xxh3_mix_step_default(data1, secret_offset, seed);
    acc[1] += xxh3_mix_step_default(data2, secret_offset + 16, seed);
    acc[0] ^= data20 + data21;
    acc[1] ^= data10 + data11;
}

inline ulong xxh3_64_len_0(ulong seed) {
    return xxh64_avalanche(seed ^ xxh3_default_secret_u64(56) ^ xxh3_default_secret_u64(64));
}

inline ulong xxh3_64_len_1_to_3(device const uchar *data, ulong len, ulong seed) {
    uint combined = xxh3_1_to_3_combined(data, len);
    ulong secret = (ulong)(xxh3_default_secret_u32(0) ^ xxh3_default_secret_u32(4));
    return xxh64_avalanche((secret + seed) ^ (ulong)combined);
}

inline ulong xxh3_64_len_4_to_8(device const uchar *data, ulong len, ulong seed) {
    ulong input_first = (ulong)read_le32_unaligned(data);
    ulong input_last = (ulong)read_le32_unaligned(data + len - 4);
    ulong modified_seed = seed ^ ((ulong)bswap32((uint)seed) << 32);
    ulong combined = input_last | (input_first << 32);
    ulong value = ((xxh3_default_secret_u64(8) ^ xxh3_default_secret_u64(16)) - modified_seed) ^ combined;
    value ^= rotl64(value, 49) ^ rotl64(value, 24);
    value *= XXH3_PRIME_MX2;
    value ^= (value >> 35) + len;
    value *= XXH3_PRIME_MX2;
    value ^= value >> 28;
    return value;
}

inline ulong xxh3_64_len_9_to_16(device const uchar *data, ulong len, ulong seed) {
    ulong input_first = read_le64_unaligned(data);
    ulong input_last = read_le64_unaligned(data + len - 8);
    ulong low = ((xxh3_default_secret_u64(24) ^ xxh3_default_secret_u64(32)) + seed) ^ input_first;
    ulong high = ((xxh3_default_secret_u64(40) ^ xxh3_default_secret_u64(48)) - seed) ^ input_last;
    U128Value product = mul64_to128(low, high);
    ulong value = len + bswap64(low) + high + xor_fold128(product);
    return xxh3_avalanche(value);
}

inline ulong xxh3_64_len_17_to_128(device const uchar *data, ulong len, ulong seed) {
    ulong acc = len * XXH64_PRIME1;

    acc += xxh3_mix_step_default(data, 0, seed);
    acc += xxh3_mix_step_default(data + len - 16, 16, seed);
    if (len > 32) {
        acc += xxh3_mix_step_default(data + 16, 32, seed);
        acc += xxh3_mix_step_default(data + len - 32, 48, seed);
        if (len > 64) {
            acc += xxh3_mix_step_default(data + 32, 64, seed);
            acc += xxh3_mix_step_default(data + len - 48, 80, seed);
            if (len > 96) {
                acc += xxh3_mix_step_default(data + 48, 96, seed);
                acc += xxh3_mix_step_default(data + len - 64, 112, seed);
            }
        }
    }

    return xxh3_avalanche(acc);
}

inline ulong xxh3_64_len_129_to_240(device const uchar *data, ulong len, ulong seed) {
    ulong acc = len * XXH64_PRIME1;
    ulong chunks = len / 16;

    for (uint i = 0; i < 8; i++) {
        acc += xxh3_mix_step_default(data + (ulong)i * 16, (ulong)i * 16, seed);
    }

    acc = xxh3_avalanche(acc);

    for (ulong i = 8; i < chunks; i++) {
        acc += xxh3_mix_step_default(data + i * 16, 3 + (i - 8) * 16, seed);
    }

    acc += xxh3_mix_step_default(data + len - 16, 119, seed);
    return xxh3_avalanche(acc);
}

inline void xxh3_large_accumulate(thread ulong acc[8],
                                  device const uchar *stripe,
                                  device const uchar *secret) {
    for (uint i = 0; i < 8; i++) {
        ulong stripe_word = read_le64_unaligned(stripe + (ulong)i * 8);
        ulong secret_word = xxh3_derived_secret_u64(secret, (ulong)i * 8);
        ulong value = stripe_word ^ secret_word;
        acc[i ^ 1] += stripe_word;
        acc[i] += (value & 0xffffffffUL) * (value >> 32);
    }
}

inline void xxh3_large_scramble(thread ulong acc[8], device const uchar *secret_end) {
    for (uint i = 0; i < 8; i++) {
        acc[i] ^= acc[i] >> 47;
        acc[i] ^= xxh3_derived_secret_u64(secret_end, (ulong)i * 8);
        acc[i] *= (ulong)XXH32_PRIME1;
    }
}

inline void xxh3_large_last_round(thread ulong acc[8],
                                  device const uchar *data,
                                  ulong len,
                                  ulong last_block_offset,
                                  ulong last_block_len,
                                  device const uchar *secret) {
    ulong full_stripes = last_block_len / 64;
    ulong stripes = (last_block_len % 64 == 0) ? (full_stripes - 1) : full_stripes;

    for (ulong i = 0; i < stripes; i++) {
        xxh3_large_accumulate(acc, data + last_block_offset + i * 64, secret + i * 8);
    }

    xxh3_large_accumulate(acc, data + len - 64, secret + 121);
}

inline ulong xxh3_large_final_merge(thread ulong acc[8],
                                    ulong init_value,
                                    device const uchar *secret,
                                    ulong secret_offset) {
    ulong result = init_value;
    for (uint i = 0; i < 4; i++) {
        ulong sa = xxh3_derived_secret_u64(secret, secret_offset + (ulong)i * 16);
        ulong sb = xxh3_derived_secret_u64(secret, secret_offset + (ulong)i * 16 + 8);
        result += xor_fold128(mul64_to128(acc[i * 2] ^ sa, acc[i * 2 + 1] ^ sb));
    }
    return xxh3_avalanche(result);
}

inline void xxh3_large_accumulators(device const uchar *data,
                                    ulong len,
                                    device const uchar *secret,
                                    thread ulong acc[8],
                                    thread ulong &last_block_offset,
                                    thread ulong &last_block_len) {
    acc[0] = (ulong)XXH32_PRIME3;
    acc[1] = XXH64_PRIME1;
    acc[2] = XXH64_PRIME2;
    acc[3] = XXH64_PRIME3;
    acc[4] = XXH64_PRIME4;
    acc[5] = (ulong)XXH32_PRIME2;
    acc[6] = XXH64_PRIME5;
    acc[7] = (ulong)XXH32_PRIME1;

    ulong full_blocks = len / 1024;
    ulong remainder = len - full_blocks * 1024;
    ulong blocks_to_process = (remainder == 0) ? (full_blocks - 1) : full_blocks;
    last_block_offset = blocks_to_process * 1024;
    last_block_len = (remainder == 0) ? 1024 : remainder;

    for (ulong block = 0; block < blocks_to_process; block++) {
        ulong block_offset = block * 1024;
        for (uint stripe = 0; stripe < 16; stripe++) {
            xxh3_large_accumulate(acc, data + block_offset + (ulong)stripe * 64, secret + (ulong)stripe * 8);
        }
        xxh3_large_scramble(acc, secret + 128);
    }
}

inline ulong xxh3_64_large(device const uchar *data, ulong len, device const uchar *secret) {
    ulong acc[8];
    ulong last_block_offset;
    ulong last_block_len;
    xxh3_large_accumulators(data, len, secret, acc, last_block_offset, last_block_len);
    xxh3_large_last_round(acc, data, len, last_block_offset, last_block_len, secret);
    return xxh3_large_final_merge(acc, len * XXH64_PRIME1, secret, 11);
}

inline ulong xxhash3_64_one(device const uchar *data,
                            ulong len,
                            ulong seed,
                            device const uchar *derived_secret) {
    if (len > 240) {
        return xxh3_64_large(data, len, derived_secret);
    }
    if (len >= 129) {
        return xxh3_64_len_129_to_240(data, len, seed);
    }
    if (len >= 17) {
        return xxh3_64_len_17_to_128(data, len, seed);
    }
    if (len >= 9) {
        return xxh3_64_len_9_to_16(data, len, seed);
    }
    if (len >= 4) {
        return xxh3_64_len_4_to_8(data, len, seed);
    }
    if (len >= 1) {
        return xxh3_64_len_1_to_3(data, len, seed);
    }
    return xxh3_64_len_0(seed);
}

inline U128Value xxh3_128_len_0(ulong seed) {
    ulong low = xxh64_avalanche(seed ^ xxh3_default_secret_u64(64) ^ xxh3_default_secret_u64(72));
    ulong high = xxh64_avalanche(seed ^ xxh3_default_secret_u64(80) ^ xxh3_default_secret_u64(88));
    return make_u128(low, high);
}

inline U128Value xxh3_128_len_1_to_3(device const uchar *data, ulong len, ulong seed) {
    uint combined = xxh3_1_to_3_combined(data, len);
    ulong low = ((ulong)(xxh3_default_secret_u32(0) ^ xxh3_default_secret_u32(4)) + seed) ^ (ulong)combined;
    uint high_input = rotl32(bswap32(combined), 13);
    ulong high = ((ulong)(xxh3_default_secret_u32(8) ^ xxh3_default_secret_u32(12)) - seed) ^ (ulong)high_input;
    return make_u128(xxh64_avalanche(low), xxh64_avalanche(high));
}

inline U128Value xxh3_128_len_4_to_8(device const uchar *data, ulong len, ulong seed) {
    ulong input_first = (ulong)read_le32_unaligned(data);
    ulong input_last = (ulong)read_le32_unaligned(data + len - 4);
    ulong modified_seed = seed ^ ((ulong)bswap32((uint)seed) << 32);
    ulong combined = input_first | (input_last << 32);
    ulong lhs = ((xxh3_default_secret_u64(16) ^ xxh3_default_secret_u64(24)) + modified_seed) ^ combined;
    ulong rhs = XXH64_PRIME1 + (len << 2);
    U128Value product = mul64_to128(lhs, rhs);
    ulong high = product.high + (product.low << 1);
    ulong low = product.low;
    low ^= high >> 3;
    low ^= low >> 35;
    low *= XXH3_PRIME_MX2;
    low ^= low >> 28;
    high = xxh3_avalanche(high);
    return make_u128(low, high);
}

inline U128Value xxh3_128_len_9_to_16(device const uchar *data, ulong len, ulong seed) {
    ulong input_first = read_le64_unaligned(data);
    ulong input_last = read_le64_unaligned(data + len - 8);
    ulong val1 = ((xxh3_default_secret_u64(32) ^ xxh3_default_secret_u64(40)) - seed) ^ input_first ^ input_last;
    ulong val2 = ((xxh3_default_secret_u64(48) ^ xxh3_default_secret_u64(56)) + seed) ^ input_last;
    U128Value product = mul64_to128(val1, XXH64_PRIME1);
    ulong low = product.low + ((len - 1) << 54);
    ulong high = product.high + (val2 & 0xffffffff00000000UL) + ((val2 & 0xffffffffUL) * (ulong)XXH32_PRIME2);
    low ^= bswap64(high);
    U128Value q = mul128_by64(low, high, XXH64_PRIME2);
    return make_u128(xxh3_avalanche(q.low), xxh3_avalanche(q.high));
}

inline U128Value xxh3_128_finalize_medium(thread ulong acc[2], ulong len, ulong seed) {
    ulong low = xxh3_avalanche(acc[0] + acc[1]);
    ulong high = acc[0] * XXH64_PRIME1
        + acc[1] * XXH64_PRIME4
        + ((len - seed) * XXH64_PRIME2);
    high = (ulong)(0UL - xxh3_avalanche(high));
    return make_u128(low, high);
}

inline U128Value xxh3_128_len_17_to_128(device const uchar *data, ulong len, ulong seed) {
    ulong acc[2];
    acc[0] = len * XXH64_PRIME1;
    acc[1] = 0;

    if (len > 32) {
        if (len > 64) {
            if (len > 96) {
                xxh3_mix_two_chunks_default(acc, data + 48, data + len - 64, 96, seed);
            }
            xxh3_mix_two_chunks_default(acc, data + 32, data + len - 48, 64, seed);
        }
        xxh3_mix_two_chunks_default(acc, data + 16, data + len - 32, 32, seed);
    }
    xxh3_mix_two_chunks_default(acc, data, data + len - 16, 0, seed);

    return xxh3_128_finalize_medium(acc, len, seed);
}

inline U128Value xxh3_128_len_129_to_240(device const uchar *data, ulong len, ulong seed) {
    ulong acc[2];
    acc[0] = len * XXH64_PRIME1;
    acc[1] = 0;

    ulong pair_count = (len / 16) / 2;
    for (uint i = 0; i < 4; i++) {
        xxh3_mix_two_chunks_default(acc, data + (ulong)i * 32, data + (ulong)i * 32 + 16, (ulong)i * 32, seed);
    }

    acc[0] = xxh3_avalanche(acc[0]);
    acc[1] = xxh3_avalanche(acc[1]);

    for (ulong i = 4; i < pair_count; i++) {
        xxh3_mix_two_chunks_default(acc, data + i * 32, data + i * 32 + 16, 3 + (i - 4) * 32, seed);
    }

    xxh3_mix_two_chunks_default(acc, data + len - 16, data + len - 32, 103, 0UL - seed);
    return xxh3_128_finalize_medium(acc, len, seed);
}

inline U128Value xxh3_128_large(device const uchar *data, ulong len, device const uchar *secret) {
    ulong acc[8];
    ulong last_block_offset;
    ulong last_block_len;
    xxh3_large_accumulators(data, len, secret, acc, last_block_offset, last_block_len);
    xxh3_large_last_round(acc, data, len, last_block_offset, last_block_len, secret);
    ulong low = xxh3_large_final_merge(acc, len * XXH64_PRIME1, secret, 11);
    ulong high = xxh3_large_final_merge(acc, ~(len * XXH64_PRIME2), secret, 117);
    return make_u128(low, high);
}

inline U128Value xxhash3_128_one(device const uchar *data,
                                 ulong len,
                                 ulong seed,
                                 device const uchar *derived_secret) {
    if (len > 240) {
        return xxh3_128_large(data, len, derived_secret);
    }
    if (len >= 129) {
        return xxh3_128_len_129_to_240(data, len, seed);
    }
    if (len >= 17) {
        return xxh3_128_len_17_to_128(data, len, seed);
    }
    if (len >= 9) {
        return xxh3_128_len_9_to_16(data, len, seed);
    }
    if (len >= 4) {
        return xxh3_128_len_4_to_8(data, len, seed);
    }
    if (len >= 1) {
        return xxh3_128_len_1_to_3(data, len, seed);
    }
    return xxh3_128_len_0(seed);
}

kernel void xxhash3_64_batch(device const uchar *input [[buffer(0)]],
                             device const MessageDesc *descs [[buffer(1)]],
                             device ulong *output [[buffer(2)]],
                             constant XxHash3Config &config [[buffer(3)]],
                             device const uchar *derived_secret [[buffer(4)]],
                             uint gid [[thread_position_in_grid]]) {
    if (gid >= config.count) {
        return;
    }

    MessageDesc desc = descs[gid];
    output[gid] = xxhash3_64_one(input + desc.offset, desc.len, config.seed, derived_secret);
}

kernel void xxhash3_128_batch(device const uchar *input [[buffer(0)]],
                              device const MessageDesc *descs [[buffer(1)]],
                              device ulong2 *output [[buffer(2)]],
                              constant XxHash3Config &config [[buffer(3)]],
                              device const uchar *derived_secret [[buffer(4)]],
                              uint gid [[thread_position_in_grid]]) {
    if (gid >= config.count) {
        return;
    }

    MessageDesc desc = descs[gid];
    U128Value hash = xxhash3_128_one(input + desc.offset, desc.len, config.seed, derived_secret);
    output[gid] = ulong2(hash.low, hash.high);
}

constant uint SHA256_K[64] = {
    0x428a2f98U, 0x71374491U, 0xb5c0fbcfU, 0xe9b5dba5U,
    0x3956c25bU, 0x59f111f1U, 0x923f82a4U, 0xab1c5ed5U,
    0xd807aa98U, 0x12835b01U, 0x243185beU, 0x550c7dc3U,
    0x72be5d74U, 0x80deb1feU, 0x9bdc06a7U, 0xc19bf174U,
    0xe49b69c1U, 0xefbe4786U, 0x0fc19dc6U, 0x240ca1ccU,
    0x2de92c6fU, 0x4a7484aaU, 0x5cb0a9dcU, 0x76f988daU,
    0x983e5152U, 0xa831c66dU, 0xb00327c8U, 0xbf597fc7U,
    0xc6e00bf3U, 0xd5a79147U, 0x06ca6351U, 0x14292967U,
    0x27b70a85U, 0x2e1b2138U, 0x4d2c6dfcU, 0x53380d13U,
    0x650a7354U, 0x766a0abbU, 0x81c2c92eU, 0x92722c85U,
    0xa2bfe8a1U, 0xa81a664bU, 0xc24b8b70U, 0xc76c51a3U,
    0xd192e819U, 0xd6990624U, 0xf40e3585U, 0x106aa070U,
    0x19a4c116U, 0x1e376c08U, 0x2748774cU, 0x34b0bcb5U,
    0x391c0cb3U, 0x4ed8aa4aU, 0x5b9cca4fU, 0x682e6ff3U,
    0x748f82eeU, 0x78a5636fU, 0x84c87814U, 0x8cc70208U,
    0x90befffaU, 0xa4506cebU, 0xbef9a3f7U, 0xc67178f2U,
};

inline uint sha256_ch(uint x, uint y, uint z) {
    return (x & y) ^ (~x & z);
}

inline uint sha256_maj(uint x, uint y, uint z) {
    return (x & y) ^ (x & z) ^ (y & z);
}

inline uint sha256_big_sigma0(uint x) {
    return rotr32(x, 2) ^ rotr32(x, 13) ^ rotr32(x, 22);
}

inline uint sha256_big_sigma1(uint x) {
    return rotr32(x, 6) ^ rotr32(x, 11) ^ rotr32(x, 25);
}

inline uint sha256_small_sigma0(uint x) {
    return rotr32(x, 7) ^ rotr32(x, 18) ^ (x >> 3);
}

inline uint sha256_small_sigma1(uint x) {
    return rotr32(x, 17) ^ rotr32(x, 19) ^ (x >> 10);
}

inline uchar sha256_padded_byte(device const uchar *data,
                                ulong len,
                                ulong padded_len,
                                ulong index) {
    if (index < len) {
        return data[index];
    }
    if (index == len) {
        return 0x80;
    }

    ulong length_start = padded_len - 8;
    if (index >= length_start) {
        ulong bit_len = len * 8;
        uint shift = (uint)((7 - (index - length_start)) * 8);
        return (uchar)((bit_len >> shift) & 0xff);
    }

    return 0;
}

inline uint sha256_padded_word(device const uchar *data,
                               ulong len,
                               ulong padded_len,
                               ulong index) {
    return ((uint)sha256_padded_byte(data, len, padded_len, index) << 24)
        | ((uint)sha256_padded_byte(data, len, padded_len, index + 1) << 16)
        | ((uint)sha256_padded_byte(data, len, padded_len, index + 2) << 8)
        | (uint)sha256_padded_byte(data, len, padded_len, index + 3);
}

inline void sha256_store_be32(device uchar *output, uint value) {
    output[0] = (uchar)(value >> 24);
    output[1] = (uchar)(value >> 16);
    output[2] = (uchar)(value >> 8);
    output[3] = (uchar)value;
}

inline void sha256_one(device const uchar *data, ulong len, device uchar *digest) {
    uint h0 = 0x6a09e667U;
    uint h1 = 0xbb67ae85U;
    uint h2 = 0x3c6ef372U;
    uint h3 = 0xa54ff53aU;
    uint h4 = 0x510e527fU;
    uint h5 = 0x9b05688cU;
    uint h6 = 0x1f83d9abU;
    uint h7 = 0x5be0cd19U;

    ulong padded_len = ((len + 9 + 63) / 64) * 64;

    for (ulong block = 0; block < padded_len; block += 64) {
        uint w[64];
        for (uint i = 0; i < 16; i++) {
            w[i] = sha256_padded_word(data, len, padded_len, block + (ulong)i * 4);
        }
        for (uint i = 16; i < 64; i++) {
            w[i] = sha256_small_sigma1(w[i - 2]) + w[i - 7]
                + sha256_small_sigma0(w[i - 15]) + w[i - 16];
        }

        uint a = h0;
        uint b = h1;
        uint c = h2;
        uint d = h3;
        uint e = h4;
        uint f = h5;
        uint g = h6;
        uint h = h7;

        for (uint i = 0; i < 64; i++) {
            uint t1 = h + sha256_big_sigma1(e) + sha256_ch(e, f, g) + SHA256_K[i] + w[i];
            uint t2 = sha256_big_sigma0(a) + sha256_maj(a, b, c);
            h = g;
            g = f;
            f = e;
            e = d + t1;
            d = c;
            c = b;
            b = a;
            a = t1 + t2;
        }

        h0 += a;
        h1 += b;
        h2 += c;
        h3 += d;
        h4 += e;
        h5 += f;
        h6 += g;
        h7 += h;
    }

    sha256_store_be32(digest + 0, h0);
    sha256_store_be32(digest + 4, h1);
    sha256_store_be32(digest + 8, h2);
    sha256_store_be32(digest + 12, h3);
    sha256_store_be32(digest + 16, h4);
    sha256_store_be32(digest + 20, h5);
    sha256_store_be32(digest + 24, h6);
    sha256_store_be32(digest + 28, h7);
}

kernel void sha256_batch(device const uchar *input [[buffer(0)]],
                         device const MessageDesc *descs [[buffer(1)]],
                         device uchar *output [[buffer(2)]],
                         constant Sha256Config &config [[buffer(3)]],
                         uint gid [[thread_position_in_grid]]) {
    if (gid >= config.count) {
        return;
    }

    MessageDesc desc = descs[gid];
    sha256_one(input + desc.offset, desc.len, output + (ulong)gid * 32);
}
