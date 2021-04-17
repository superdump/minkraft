/*
 * MIT License
 *
 * Copyright (c) 2020 bonsairobo
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 *
 */

use building_blocks::{core::prelude::*, storage::Array3x1};

// TODO: it would be nice if all crates could share this module, but it causes this issue:
// https://github.com/rust-lang/cargo/issues/6765

pub fn sphere_bit_array<T>(
    array_edge_length: i32,
    inner_value: T,
    outer_value: T,
) -> (Array3x1<T>, i32)
where
    T: Copy,
{
    let array_radius = array_edge_length / 2;
    let sphere_radius = array_radius - 1;
    let array_extent = Extent3i::from_min_and_shape(
        Point3i::fill(-array_radius),
        Point3i::fill(array_edge_length),
    );

    let map = Array3x1::fill_with(array_extent, |p| {
        if p.norm() < sphere_radius as f32 {
            inner_value
        } else {
            outer_value
        }
    });

    (map, sphere_radius)
}
