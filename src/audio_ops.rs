#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub fn planar_to_interleaved(left: &[f32], right: &[f32]) -> Vec<f32> {
    use std::arch::x86_64::*;

    assert_eq!(left.len(), right.len(), "Left and right channels must have same length");

    let len = left.len();
    let mut interleaved = vec![0f32; len * 2];

    // Process 8 samples at a time with AVX (256-bit)
    if is_x86_feature_detected!("avx") {
        let chunks = len / 8;

        unsafe {
            for i in 0..chunks {
                let l_offset = i * 8;
                let r_offset = i * 8;

                // Load 8 floats from each channel
                let l = _mm256_loadu_ps(left.as_ptr().add(l_offset));
                let r = _mm256_loadu_ps(right.as_ptr().add(r_offset));

                // Interleave: unpack low and high parts
                let l_low = _mm256_castps256_ps128(l);
                let l_high = _mm256_extractf128_ps(l, 1);
                let r_low = _mm256_castps256_ps128(r);
                let r_high = _mm256_extractf128_ps(r, 1);

                let interleaved_0 = _mm_unpacklo_ps(l_low, r_low);
                let interleaved_1 = _mm_unpackhi_ps(l_low, r_low);
                let interleaved_2 = _mm_unpacklo_ps(l_high, r_high);
                let interleaved_3 = _mm_unpackhi_ps(l_high, r_high);

                // Store results
                let out_offset = i * 16;
                _mm_storeu_ps(interleaved.as_mut_ptr().add(out_offset), interleaved_0);
                _mm_storeu_ps(interleaved.as_mut_ptr().add(out_offset + 4), interleaved_1);
                _mm_storeu_ps(interleaved.as_mut_ptr().add(out_offset + 8), interleaved_2);
                _mm_storeu_ps(interleaved.as_mut_ptr().add(out_offset + 12), interleaved_3);
            }
        }

        // Handle remaining samples
        for i in chunks * 8..len {
            let out_idx = i * 2;
            interleaved[out_idx] = left[i];
            interleaved[out_idx + 1] = right[i];
        }
    } else {
        return planar_to_interleaved_scalar(left, right);
    }

    interleaved
}

#[cfg(all(feature = "simd", target_arch = "aarch64"))]
pub fn planar_to_interleaved(left: &[f32], right: &[f32]) -> Vec<f32> {
    use std::arch::aarch64::*;

    assert_eq!(left.len(), right.len(), "Left and right channels must have same length");

    let len = left.len();
    let mut interleaved = vec![0f32; len * 2];

    // Process 4 samples at a time with NEON (128-bit)
    let chunks = len / 4;

    unsafe {
        for i in 0..chunks {
            let l_offset = i * 4;
            let r_offset = i * 4;

            // Load 4 floats from each channel
            let l = vld1q_f32(left.as_ptr().add(l_offset));
            let r = vld1q_f32(right.as_ptr().add(r_offset));

            // Interleave using zip
            let result = vzip1q_f32(l, r);
            let result2 = vzip2q_f32(l, r);

            // Store results
            let out_offset = i * 8;
            vst1q_f32(interleaved.as_mut_ptr().add(out_offset), result);
            vst1q_f32(interleaved.as_mut_ptr().add(out_offset + 4), result2);
        }
    }

    // Handle remaining samples
    for i in chunks * 4..len {
        let out_idx = i * 2;
        interleaved[out_idx] = left[i];
        interleaved[out_idx + 1] = right[i];
    }

    interleaved
}

#[cfg(all(feature = "simd", not(any(target_arch = "x86_64", target_arch = "aarch64"))))]
pub fn planar_to_interleaved(left: &[f32], right: &[f32]) -> Vec<f32> {
    planar_to_interleaved_scalar(left, right)
}

#[cfg(not(feature = "simd"))]
pub fn planar_to_interleaved(left: &[f32], right: &[f32]) -> Vec<f32> {
    planar_to_interleaved_scalar(left, right)
}

pub fn planar_to_interleaved_scalar(left: &[f32], right: &[f32]) -> Vec<f32> {
    assert_eq!(left.len(), right.len(), "Left and right channels must have same length");

    let mut interleaved = Vec::with_capacity(left.len() * 2);
    for (l, r) in left.iter().zip(right.iter()) {
        interleaved.push(*l);
        interleaved.push(*r);
    }
    interleaved
}

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub fn interleaved_to_planar(interleaved: &[f32]) -> [Vec<f32>; 2] {
    use std::arch::x86_64::*;

    assert_eq!(interleaved.len() % 2, 0, "Interleaved buffer must have even length");

    let frame_count = interleaved.len() / 2;
    let mut left = vec![0f32; frame_count];
    let mut right = vec![0f32; frame_count];

    if is_x86_feature_detected!("avx") {
        let chunks = frame_count / 8;

        unsafe {
            for i in 0..chunks {
                let in_offset = i * 16;

                // Load 16 interleaved samples (8 frames)
                let data0 = _mm_loadu_ps(interleaved.as_ptr().add(in_offset));
                let data1 = _mm_loadu_ps(interleaved.as_ptr().add(in_offset + 4));
                let data2 = _mm_loadu_ps(interleaved.as_ptr().add(in_offset + 8));
                let data3 = _mm_loadu_ps(interleaved.as_ptr().add(in_offset + 12));

                // De-interleave using shuffle
                let l0 = _mm_shuffle_ps(data0, data1, 0b10001000); // L0 L1 L2 L3
                let r0 = _mm_shuffle_ps(data0, data1, 0b11011101); // R0 R1 R2 R3
                let l1 = _mm_shuffle_ps(data2, data3, 0b10001000); // L4 L5 L6 L7
                let r1 = _mm_shuffle_ps(data2, data3, 0b11011101); // R4 R5 R6 R7

                // Store results
                let out_offset = i * 8;
                _mm_storeu_ps(left.as_mut_ptr().add(out_offset), l0);
                _mm_storeu_ps(left.as_mut_ptr().add(out_offset + 4), l1);
                _mm_storeu_ps(right.as_mut_ptr().add(out_offset), r0);
                _mm_storeu_ps(right.as_mut_ptr().add(out_offset + 4), r1);
            }
        }

        // Handle remaining samples
        for i in chunks * 8..frame_count {
            let idx = i * 2;
            left[i] = interleaved[idx];
            right[i] = interleaved[idx + 1];
        }
    } else {
        return interleaved_to_planar_scalar(interleaved);
    }

    [left, right]
}

#[cfg(all(feature = "simd", target_arch = "aarch64"))]
pub fn interleaved_to_planar(interleaved: &[f32]) -> [Vec<f32>; 2] {
    use std::arch::aarch64::*;

    assert_eq!(interleaved.len() % 2, 0, "Interleaved buffer must have even length");

    let frame_count = interleaved.len() / 2;
    let mut left = vec![0f32; frame_count];
    let mut right = vec![0f32; frame_count];

    let chunks = frame_count / 4;

    unsafe {
        for i in 0..chunks {
            let in_offset = i * 8;

            // Load 8 interleaved samples (4 frames)
            let data1 = vld1q_f32(interleaved.as_ptr().add(in_offset));
            let data2 = vld1q_f32(interleaved.as_ptr().add(in_offset + 4));

            // De-interleave using unzip
            let result = vuzp1q_f32(data1, data2);
            let result2 = vuzp2q_f32(data1, data2);

            // Store results
            let out_offset = i * 4;
            vst1q_f32(left.as_mut_ptr().add(out_offset), result);
            vst1q_f32(right.as_mut_ptr().add(out_offset), result2);
        }
    }

    // Handle remaining samples
    for i in chunks * 4..frame_count {
        let idx = i * 2;
        left[i] = interleaved[idx];
        right[i] = interleaved[idx + 1];
    }

    [left, right]
}

#[cfg(all(feature = "simd", not(any(target_arch = "x86_64", target_arch = "aarch64"))))]
pub fn interleaved_to_planar(interleaved: &[f32]) -> [Vec<f32>; 2] {
    interleaved_to_planar_scalar(interleaved)
}

#[cfg(not(feature = "simd"))]
pub fn interleaved_to_planar(interleaved: &[f32]) -> [Vec<f32>; 2] {
    interleaved_to_planar_scalar(interleaved)
}

pub fn interleaved_to_planar_scalar(interleaved: &[f32]) -> [Vec<f32>; 2] {
    assert_eq!(interleaved.len() % 2, 0, "Interleaved buffer must have even length");

    let frame_count = interleaved.len() / 2;
    let mut left = Vec::with_capacity(frame_count);
    let mut right = Vec::with_capacity(frame_count);

    for chunk in interleaved.chunks(2) {
        left.push(chunk[0]);
        right.push(chunk[1]);
    }

    [left, right]
}

/// Multiply buffer by weights and accumulate into output
/// output[i] += buffer[i] * weight[i]
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub fn weighted_accumulate(buffer: &[f32], weights: &[f32], output: &mut [f32]) {
    use std::arch::x86_64::*;

    assert_eq!(buffer.len(), weights.len());
    assert_eq!(buffer.len(), output.len());

    if is_x86_feature_detected!("avx") {
        let len = buffer.len();
        let chunks = len / 8;

        unsafe {
            for i in 0..chunks {
                let offset = i * 8;

                let buf = _mm256_loadu_ps(buffer.as_ptr().add(offset));
                let weight = _mm256_loadu_ps(weights.as_ptr().add(offset));
                let out = _mm256_loadu_ps(output.as_ptr().add(offset));

                let result = _mm256_add_ps(out, _mm256_mul_ps(buf, weight));
                _mm256_storeu_ps(output.as_mut_ptr().add(offset), result);
            }
        }

        // Handle remaining samples
        for i in chunks * 8..len {
            output[i] += buffer[i] * weights[i];
        }
    } else {
        weighted_accumulate_scalar(buffer, weights, output);
    }
}

#[cfg(all(feature = "simd", target_arch = "aarch64"))]
pub fn weighted_accumulate(buffer: &[f32], weights: &[f32], output: &mut [f32]) {
    use std::arch::aarch64::*;

    assert_eq!(buffer.len(), weights.len());
    assert_eq!(buffer.len(), output.len());

    let len = buffer.len();
    let chunks = len / 4;

    unsafe {
        for i in 0..chunks {
            let offset = i * 4;

            let buf = vld1q_f32(buffer.as_ptr().add(offset));
            let weight = vld1q_f32(weights.as_ptr().add(offset));
            let out = vld1q_f32(output.as_ptr().add(offset));

            let result = vmlaq_f32(out, buf, weight);  // out + buf * weight
            vst1q_f32(output.as_mut_ptr().add(offset), result);
        }
    }

    // Handle remaining samples
    for i in chunks * 4..len {
        output[i] += buffer[i] * weights[i];
    }
}

#[cfg(all(feature = "simd", not(any(target_arch = "x86_64", target_arch = "aarch64"))))]
pub fn weighted_accumulate(buffer: &[f32], weights: &[f32], output: &mut [f32]) {
    weighted_accumulate_scalar(buffer, weights, output);
}

#[cfg(not(feature = "simd"))]
pub fn weighted_accumulate(buffer: &[f32], weights: &[f32], output: &mut [f32]) {
    weighted_accumulate_scalar(buffer, weights, output);
}

pub fn weighted_accumulate_scalar(buffer: &[f32], weights: &[f32], output: &mut [f32]) {
    assert_eq!(buffer.len(), weights.len());
    assert_eq!(buffer.len(), output.len());

    for i in 0..buffer.len() {
        output[i] += buffer[i] * weights[i];
    }
}

/// Accumulate weights: output[i] += weight[i]
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub fn accumulate_weights(weights: &[f32], output: &mut [f32]) {
    use std::arch::x86_64::*;

    assert_eq!(weights.len(), output.len());

    if is_x86_feature_detected!("avx") {
        let len = weights.len();
        let chunks = len / 8;

        unsafe {
            for i in 0..chunks {
                let offset = i * 8;

                let weight = _mm256_loadu_ps(weights.as_ptr().add(offset));
                let out = _mm256_loadu_ps(output.as_ptr().add(offset));

                let result = _mm256_add_ps(out, weight);
                _mm256_storeu_ps(output.as_mut_ptr().add(offset), result);
            }
        }

        for i in chunks * 8..len {
            output[i] += weights[i];
        }
    } else {
        accumulate_weights_scalar(weights, output);
    }
}

#[cfg(all(feature = "simd", target_arch = "aarch64"))]
pub fn accumulate_weights(weights: &[f32], output: &mut [f32]) {
    use std::arch::aarch64::*;

    assert_eq!(weights.len(), output.len());

    let len = weights.len();
    let chunks = len / 4;

    unsafe {
        for i in 0..chunks {
            let offset = i * 4;

            let weight = vld1q_f32(weights.as_ptr().add(offset));
            let out = vld1q_f32(output.as_ptr().add(offset));

            let result = vaddq_f32(out, weight);
            vst1q_f32(output.as_mut_ptr().add(offset), result);
        }
    }

    for i in chunks * 4..len {
        output[i] += weights[i];
    }
}

#[cfg(all(feature = "simd", not(any(target_arch = "x86_64", target_arch = "aarch64"))))]
pub fn accumulate_weights(weights: &[f32], output: &mut [f32]) {
    accumulate_weights_scalar(weights, output);
}

#[cfg(not(feature = "simd"))]
pub fn accumulate_weights(weights: &[f32], output: &mut [f32]) {
    accumulate_weights_scalar(weights, output);
}

pub fn accumulate_weights_scalar(weights: &[f32], output: &mut [f32]) {
    assert_eq!(weights.len(), output.len());

    for i in 0..weights.len() {
        output[i] += weights[i];
    }
}

/// Normalize buffer by weights: buffer[i] /= weights[i] (if weight > 0)
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub fn normalize_by_weights(buffer: &mut [f32], weights: &[f32]) {
    use std::arch::x86_64::*;

    assert_eq!(buffer.len(), weights.len());

    if is_x86_feature_detected!("avx") {
        let len = buffer.len();
        let chunks = len / 8;

        unsafe {
            let zero = _mm256_setzero_ps();

            for i in 0..chunks {
                let offset = i * 8;

                let buf = _mm256_loadu_ps(buffer.as_ptr().add(offset));
                let weight = _mm256_loadu_ps(weights.as_ptr().add(offset));

                // Check if weight > 0
                let mask = _mm256_cmp_ps(weight, zero, _CMP_GT_OQ);

                // Divide where weight > 0
                let result = _mm256_div_ps(buf, weight);

                // Blend: keep original where weight == 0, use result where weight > 0
                let blended = _mm256_blendv_ps(buf, result, mask);

                _mm256_storeu_ps(buffer.as_mut_ptr().add(offset), blended);
            }
        }

        for i in chunks * 8..len {
            let w = weights[i];
            if w > 0.0 {
                buffer[i] /= w;
            }
        }
    } else {
        normalize_by_weights_scalar(buffer, weights);
    }
}

#[cfg(all(feature = "simd", target_arch = "aarch64"))]
pub fn normalize_by_weights(buffer: &mut [f32], weights: &[f32]) {
    use std::arch::aarch64::*;

    assert_eq!(buffer.len(), weights.len());

    let len = buffer.len();
    let chunks = len / 4;

    unsafe {
        let zero = vdupq_n_f32(0.0);

        for i in 0..chunks {
            let offset = i * 4;

            let buf = vld1q_f32(buffer.as_ptr().add(offset));
            let weight = vld1q_f32(weights.as_ptr().add(offset));

            // Check if weight > 0
            let mask = vcgtq_f32(weight, zero);

            // Divide where weight > 0
            let result = vdivq_f32(buf, weight);

            // Blend using mask
            let blended = vbslq_f32(mask, result, buf);

            vst1q_f32(buffer.as_mut_ptr().add(offset), blended);
        }
    }

    for i in chunks * 4..len {
        let w = weights[i];
        if w > 0.0 {
            buffer[i] /= w;
        }
    }
}

#[cfg(all(feature = "simd", not(any(target_arch = "x86_64", target_arch = "aarch64"))))]
pub fn normalize_by_weights(buffer: &mut [f32], weights: &[f32]) {
    normalize_by_weights_scalar(buffer, weights);
}

#[cfg(not(feature = "simd"))]
pub fn normalize_by_weights(buffer: &mut [f32], weights: &[f32]) {
    normalize_by_weights_scalar(buffer, weights);
}

pub fn normalize_by_weights_scalar(buffer: &mut [f32], weights: &[f32]) {
    assert_eq!(buffer.len(), weights.len());

    for i in 0..buffer.len() {
        let w = weights[i];
        if w > 0.0 {
            buffer[i] /= w;
        }
    }
}
