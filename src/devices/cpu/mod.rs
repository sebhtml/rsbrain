use std::f32::consts::E;
pub mod slice;
use cblas::{Layout, Transpose};
use rand::{distributions::Uniform, thread_rng, Rng};
extern crate cblas_sys as ffi;
use crate::{error, slice::DevSliceEnum, Error, ErrorEnum, Tensor, EPSILON};

use self::slice::CpuDevSlice;

use super::DeviceInterface;
extern crate blas_src;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct CpuDevice {}

impl Default for CpuDevice {
    fn default() -> Self {
        Self {}
    }
}

impl DeviceInterface for CpuDevice {
    fn gemm(
        &self,
        transa: bool,
        transb: bool,
        m: i32,
        n: i32,
        k: i32,
        alpha: f32,
        a: *const f32,
        lda: i32,
        b: *const f32,
        ldb: i32,
        beta: f32,
        c: *mut f32,
        ldc: i32,
    ) -> Result<(), Error> {
        let layout = Layout::ColumnMajor;
        let transa = match transa {
            false => Transpose::None,
            true => Transpose::Ordinary,
        };
        let transb = match transb {
            false => Transpose::None,
            true => Transpose::Ordinary,
        };

        unsafe {
            ffi::cblas_sgemm(
                layout.into(),
                transa.into(),
                transb.into(),
                m,
                n,
                k,
                alpha,
                a,
                lda,
                b,
                ldb,
                beta,
                c,
                ldc,
            )
        }
        Ok(())
    }

    fn dot(&self, x: &Tensor, y: &Tensor, output: &Tensor) -> Result<(), Error> {
        let n = x.len() as i32;
        let incx = 1;
        let incy = 1;
        let x = x.as_ptr();
        let y = y.as_ptr();
        let result = unsafe { ffi::cblas_sdot(n, x, incx, y, incy) };
        let output = output.as_mut_ptr();
        unsafe {
            *output = result;
        };
        Ok(())
    }

    fn copy(&self, n: i32, x: *const f32, incx: i32, y: *mut f32, incy: i32) -> Result<(), Error> {
        unsafe { ffi::cblas_scopy(n, x, incx, y, incy) }
        Ok(())
    }

    fn axpy(
        &self,
        n: i32,
        alpha: f32,
        x: *const f32,
        incx: i32,
        y: *mut f32,
        incy: i32,
    ) -> Result<(), Error> {
        unsafe { ffi::cblas_saxpy(n, alpha, x, incx, y, incy) }
        Ok(())
    }

    fn scalar_mul(&self, alpha: &Tensor, x: &Tensor) -> Result<(), Error> {
        let n = x.len() as i32;
        let x = x.as_mut_ptr();
        let incx = 1;
        let alpha = alpha.get_values()?;
        let alpha = alpha[0];
        unsafe { ffi::cblas_sscal(n, alpha, x, incx) }
        Ok(())
    }

    fn scalar_add(&self, alpha: &Tensor, x: &Tensor) -> Result<(), Error> {
        let n = x.len();
        let x = x.as_mut_ptr();
        let alpha = alpha.as_ptr();
        for i in 0..n {
            unsafe {
                *x.add(i) += *alpha;
            }
        }
        Ok(())
    }

    fn slice(&self, n: i32) -> Result<DevSliceEnum, Error> {
        let len = n as usize;
        let values = vec![0.0; len];
        let slice = DevSliceEnum::CpuDevSlice(CpuDevSlice::new(values));
        Ok(slice)
    }

    fn softmax(&self, input: &Tensor, output: &Tensor) -> Result<(), Error> {
        let rows = input.rows() as i32;
        let cols = input.cols() as i32;
        let input = input.as_ptr();
        let output = output.as_mut_ptr();
        CpuDevice::_softmax(rows, cols, input, output)
    }

    fn sum(&self, _input: &Tensor, _output: &Tensor) -> Result<(), Error> {
        todo!()
    }

    fn mul(&self, left: &Tensor, right: &Tensor, result: &Tensor) -> Result<(), Error> {
        if left.size() != right.size() {
            return Err(error!(ErrorEnum::IncompatibleTensorShapes));
        }

        let len = left.len();
        debug_assert_eq!(result.size(), left.size());

        let result_ptr = result.as_mut_ptr();
        let left_ptr = left.as_ptr();
        let right_ptr = right.as_ptr();

        unsafe {
            let mut index = 0;
            while index < len {
                let left_cell = left_ptr.add(index);
                let right_cell = right_ptr.add(index);
                let result_cell = result_ptr.add(index);
                let left = *left_cell;
                let right = *right_cell;
                let value = left * right;
                *result_cell = value;
                index += 1;
            }
        }
        Ok(())
    }

    fn sigmoid(&self, input: &Tensor, output: &Tensor) -> Result<(), Error> {
        let rows = input.rows();
        let cols = input.cols();
        let values = input.as_ptr();
        let result_values = output.as_mut_ptr();
        let mut row = 0;
        while row < rows {
            let mut col = 0;
            while col < cols {
                let x = unsafe { values.add(input.index(row, col)) };
                let y = sigmoid(unsafe { *x });
                unsafe { *result_values.add(output.index(row, col)) = y };
                col += 1;
            }
            row += 1;
        }

        Ok(())
    }

    fn sqrt(&self, input: &Tensor, output: &Tensor) -> Result<(), Error> {
        let rows = input.rows();
        let cols = input.cols();
        let values = input.as_ptr();
        let result_values = output.as_mut_ptr();
        let mut row = 0;
        while row < rows {
            let mut col = 0;
            while col < cols {
                let x = values.wrapping_add(input.index(row, col));
                let y = unsafe { *x }.sqrt();
                unsafe { *result_values.wrapping_add(output.index(row, col)) = y };
                col += 1;
            }
            row += 1;
        }
        Ok(())
    }

    fn clip(
        &self,
        min: &Tensor,
        max: &Tensor,
        input: &Tensor,
        output: &Tensor,
    ) -> Result<(), Error> {
        let n = input.len();
        let input = input.as_ptr();
        let output = output.as_mut_ptr();
        let min = unsafe { *min.as_ptr() };
        let max = unsafe { *max.as_ptr() };
        for idx in 0..n {
            let x = unsafe { *input.add(idx) };
            let x = x.max(min);
            let x = x.min(max);
            unsafe { *output.add(idx) = x };
        }
        Ok(())
    }

    fn div(&self, left: &Tensor, right: &Tensor, result: &Tensor) -> Result<(), Error> {
        if left.size() != right.size() {
            return Err(error!(ErrorEnum::IncompatibleTensorShapes));
        }

        let len = left.len();
        debug_assert_eq!(result.size(), left.size());

        let result_ptr = result.as_mut_ptr();
        let left_ptr = left.as_ptr();
        let right_ptr = right.as_ptr();

        unsafe {
            let mut index = 0;
            while index < len {
                let left_cell = left_ptr.add(index);
                let right_cell = right_ptr.add(index);
                let result_cell = result_ptr.add(index);
                let left = *left_cell;
                let right = *right_cell;
                let value = left / right;
                *result_cell = value;
                index += 1;
            }
        }

        Ok(())
    }

    fn cross_entropy_loss(
        &self,
        expected: &Tensor,
        actual: &Tensor,
        loss: &Tensor,
    ) -> Result<(), Error> {
        debug_assert_eq!(actual.size(), expected.size());
        let p = expected;
        let q = actual;
        if p.size() != q.size() {
            println!("Incompatible sizes");
            println!("p {}", p);
            println!("q {}", q);
            return Err(error!(ErrorEnum::IncompatibleTensorShapes));
        }
        let rows = p.rows();
        let cols = p.cols();
        let mut row = 0;
        let p_values = p.as_ptr();
        let q_values = q.as_ptr();
        let mut sum = 0.0;
        while row < rows {
            let mut col = 0;
            while col < cols {
                let p_i = unsafe { *p_values.add(p.index(row, col)) };
                let q_i = unsafe { *q_values.add(q.index(row, col)) };
                sum += p_i * f32::ln(q_i + EPSILON);
                col += 1;
            }
            row += 1;
        }

        debug_assert!(sum.is_finite());
        let loss_value = -sum;
        unsafe { *loss.as_mut_ptr() = loss_value };
        Ok(())
    }

    fn reduce_square_sum(
        &self,
        expected: &Tensor,
        actual: &Tensor,
        loss: &Tensor,
    ) -> Result<(), Error> {
        if expected.size() != actual.size() {
            return Err(error!(ErrorEnum::IncompatibleTensorShapes));
        }
        let expected_values = expected.get_values()?;
        let actual_values = actual.get_values()?;
        let mut loss_value = 0.0;
        for i in 0..expected_values.len() {
            let expected = expected_values[i];
            let actual = actual_values[i];
            let diff = expected - actual;
            loss_value += diff * diff;
        }

        loss.set_values(vec![loss_value; 1])?;
        Ok(())
    }

    fn transpose(&self, input: &Tensor, output: &Tensor) -> Result<(), Error> {
        let self_values = input.get_values()?;
        let mut other_values = output.get_values()?;
        let rows = input.rows();
        let cols = input.cols();
        let mut row = 0;
        while row < rows {
            let mut col = 0;
            while col < cols {
                let value = self_values[input.index(row, col)];
                other_values[output.index(col, row)] = value;
                col += 1;
            }
            row += 1;
        }
        output.set_values(other_values)
    }

    fn bernoulli(&self, input: &Tensor, output: &Tensor) -> Result<(), Error> {
        let len = input.len();
        let output_ptr = output.as_mut_ptr();
        let mut rng = thread_rng();
        let uniform = Uniform::new(0.0, 1.0);

        let input_ptr = input.as_ptr();
        for i in 0..len {
            let probability = unsafe { *input_ptr.add(i) };
            let random_number = if rng.sample(uniform) <= probability {
                1.0
            } else {
                0.0
            };
            unsafe { *output_ptr.add(i) = random_number };
        }
        Ok(())
    }
}

impl CpuDevice {
    pub fn _softmax(
        rows: i32,
        cols: i32,
        input: *const f32,
        output: *mut f32,
    ) -> Result<(), Error> {
        let rows = rows as usize;
        let cols = cols as usize;
        let mut row = 0;
        while row < rows {
            // Find max

            let mut max = unsafe { *input.add(row * cols + 0) };
            let mut col = 0;
            while col < cols {
                let x = unsafe { *input.add(row * cols + col) };
                max = max.max(x);
                col += 1;
            }

            // For each value:
            // 1. substract the max
            // 2. compute E^x
            // 3. add result to sum
            let mut sum = 0.0;
            let mut col = 0;
            while col < cols {
                let x = unsafe { *input.add(row * cols + col) };
                debug_assert_eq!(false, x.is_nan());
                let y = E.powf(x - max);
                debug_assert_eq!(false, y.is_nan(), "x: {}, max: {}, y: {}", x, max, y,);
                unsafe { *output.add(row * cols + col) = y };
                sum += y;
                col += 1;
            }

            // Divide every value by sum.
            let mut col = 0;
            while col < cols {
                let x = unsafe { *output.add(row * cols + col) };
                debug_assert_eq!(false, x.is_nan());
                debug_assert_ne!(0.0, sum);
                let y = x / sum;
                debug_assert_eq!(false, y.is_nan());
                unsafe { *output.add(row * cols + col) = y };
                col += 1;
            }
            row += 1;
        }

        Ok(())
    }
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + E.powf(-x))
}
