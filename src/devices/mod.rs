mod cpu;
use crate::Error;
use std::{
    borrow::Borrow,
    cell::RefCell,
    collections::{HashMap, LinkedList},
    mem::swap,
    ops::Deref,
    rc::Rc,
};

pub use cpu::*;
#[cfg(feature = "cuda")]
mod cuda;
#[cfg(feature = "cuda")]
pub use cuda::*;

use crate::{OperatorTrait, Tensor, TensorF32};
mod buffer;
pub use buffer::*;

pub struct MemoryInfo {
    pub used: usize,
    pub free: usize,
    pub total: usize,
}

pub trait DeviceInterface {
    ///  SGEMM  performs one of the matrix-matrix operations
    /// https://netlib.org/lapack/explore-html-3.6.1/db/dc9/group__single__blas__level3_gafe51bacb54592ff5de056acabd83c260.html
    ///
    /// C := alpha * op(A) * op(B) + beta * C,
    ///
    /// where  op(X) is one of
    ///    op(X) = X   or   op(X) = X^T,
    ///
    /// alpha and beta are scalars.
    /// A, B and C are matrices.
    ///
    /// op(A) is an m by k matrix
    /// op(B) is a k by n matrix
    /// C is an m by n matrix.
    ///
    fn sgemm(
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
    ) -> Result<(), Error>;

    /// SAXPY constant times a vector plus a vector.
    /// y = alpha * x + y
    fn saxpy(
        &self,
        n: i32,
        alpha: f32,
        x: &TensorF32,
        incx: i32,
        y: &mut TensorF32,
        incy: i32,
    ) -> Result<(), Error>;

    /// SDOT forms the dot product of two vectors.
    fn sdot(
        &self,
        n: i32,
        x: *const f32,
        incx: i32,
        y: *const f32,
        incy: i32,
    ) -> Result<f32, Error>;

    /// SCOPY copies a vector, x, to a vector, y.
    fn scopy(
        &self,
        n: i32,
        x: &TensorF32,
        incx: i32,
        y: &mut TensorF32,
        incy: i32,
    ) -> Result<(), Error>;

    /// SSCAL scales a vector by a constant.
    fn sscal(&self, n: i32, alpha: f32, x: &mut TensorF32, incx: i32) -> Result<(), Error>;
}

#[derive(Clone, Debug)]
pub struct Device {
    used: Rc<RefCell<usize>>,
    tensors_with_requires_grad: Rc<RefCell<Vec<Tensor>>>,
    device: Rc<DeviceEnum>,
    available_buffers: Rc<RefCell<HashMap<usize, LinkedList<DevBuffer>>>>,
}

#[derive(Debug)]
pub enum DeviceEnum {
    Cpu(CpuDevice),
    #[cfg(feature = "cuda")]
    Cuda(CudaDevice),
}

impl Default for Device {
    fn default() -> Self {
        Self::cpu()
    }
}

impl Device {
    pub fn new(device: DeviceEnum) -> Self {
        Self {
            used: Default::default(),
            tensors_with_requires_grad: Rc::new(RefCell::new(vec![])),
            device: Rc::new(device),
            available_buffers: Default::default(),
        }
    }

    pub fn cpu() -> Self {
        Self::new(DeviceEnum::Cpu(CpuDevice::default()))
    }

    pub fn recycle(&self, len: usize, buffer: &mut DevBuffer) {
        let mut recycled_buffer = DevBuffer::new(self, 0);
        swap(&mut recycled_buffer, buffer);

        let available_buffers: &mut HashMap<_, _> =
            &mut self.available_buffers.deref().borrow_mut();
        let entry = available_buffers.entry(len);
        entry.or_default().push_back(recycled_buffer)
    }

    pub fn get_memory_info(&self) -> Result<MemoryInfo, Error> {
        Ok(MemoryInfo {
            used: *self.used.deref().borrow(),
            free: 0,
            total: 0,
        })
    }

    #[cfg(feature = "cuda")]
    pub fn cuda() -> Result<Self, Error> {
        match CudaDevice::try_default() {
            Ok(cublas) => Ok(Self::new(DeviceEnum::Cuda(cublas))),
            Err(error) => Err(error),
        }
    }

    pub fn tensor_f32(&self, rows: usize, cols: usize, values: Vec<f32>) -> TensorF32 {
        TensorF32::new(rows, cols, values, self)
    }

    pub fn tensor(
        &self,
        operator: Rc<dyn OperatorTrait>,
        inputs: &[Tensor],
        rows: usize,
        cols: usize,
        values: Vec<f32>,
        requires_grad: bool,
    ) -> Tensor {
        let len = rows * cols;
        let tensor = Tensor::new(
            self,
            operator,
            inputs,
            Rc::new(RefCell::new(Self::tensor_f32(&self, rows, cols, values))),
            Rc::new(RefCell::new(Self::tensor_f32(
                &self,
                rows,
                cols,
                vec![0.0; len],
            ))),
        );
        if requires_grad {
            self.tensors_with_requires_grad
                .deref()
                .borrow_mut()
                .push(tensor.clone())
        }
        tensor
    }

    pub fn tensors_with_requires_grad(&self) -> &Rc<RefCell<Vec<Tensor>>> {
        &self.tensors_with_requires_grad
    }

    pub fn zero_grad(&self) -> Result<(), Error> {
        let gradients: &[Tensor] = &self.tensors_with_requires_grad().deref().borrow();
        for gradient in gradients {
            let gradient: &mut TensorF32 = &mut gradient.gradient().deref().borrow_mut();
            TensorF32::scalar_mul(0.0, gradient)?;
        }
        Ok(())
    }

    pub fn buffer(&self, len: usize) -> DevBuffer {
        let recycled = self
            .available_buffers
            .deref()
            .borrow_mut()
            .get_mut(&len)
            .map(|x| x.pop_back())
            .flatten();
        match recycled {
            Some(buffer) => {
                //println!("Recycled buffer with length {}", len);
                buffer
            }
            None => {
                let used: &mut usize = &mut self.used.deref().borrow_mut();
                *used += len;
                DevBuffer::new(self, len)
            }
        }
    }
}

impl DeviceInterface for Device {
    fn sgemm(
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
        match self.device.borrow() {
            DeviceEnum::Cpu(device) => {
                device.sgemm(transa, transb, m, n, k, alpha, a, lda, b, ldb, beta, c, ldc)
            }
            #[cfg(feature = "cuda")]
            DeviceEnum::Cuda(device) => {
                device.sgemm(transa, transb, m, n, k, alpha, a, lda, b, ldb, beta, c, ldc)
            }
        }
    }

    fn sdot(
        &self,
        n: i32,
        x: *const f32,
        incx: i32,
        y: *const f32,
        incy: i32,
    ) -> Result<f32, Error> {
        match self.device.borrow() {
            DeviceEnum::Cpu(device) => device.sdot(n, x, incx, y, incy),
            #[cfg(feature = "cuda")]
            DeviceEnum::Cuda(device) => device.sdot(n, x, incx, y, incy),
        }
    }

    fn scopy(
        &self,
        n: i32,
        x: &TensorF32,
        incx: i32,
        y: &mut TensorF32,
        incy: i32,
    ) -> Result<(), Error> {
        match self.device.borrow() {
            DeviceEnum::Cpu(device) => device.scopy(n, x, incx, y, incy),
            #[cfg(feature = "cuda")]
            DeviceEnum::Cuda(device) => device.scopy(n, x, incx, y, incy),
        }
    }

    fn saxpy(
        &self,
        n: i32,
        alpha: f32,
        x: &TensorF32,
        incx: i32,
        y: &mut TensorF32,
        incy: i32,
    ) -> Result<(), Error> {
        match self.device.borrow() {
            DeviceEnum::Cpu(device) => device.saxpy(n, alpha, x, incx, y, incy),
            #[cfg(feature = "cuda")]
            DeviceEnum::Cuda(device) => device.saxpy(n, alpha, x, incx, y, incy),
        }
    }

    fn sscal(&self, n: i32, alpha: f32, x: &mut TensorF32, incx: i32) -> Result<(), Error> {
        match self.device.borrow() {
            DeviceEnum::Cpu(device) => device.sscal(n, alpha, x, incx),
            #[cfg(feature = "cuda")]
            DeviceEnum::Cuda(device) => device.sscal(n, alpha, x, incx),
        }
    }
}
