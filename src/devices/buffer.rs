use crate::error;
use crate::Device;
use crate::DeviceInterface;
use crate::Error;
use crate::ErrorEnum;

#[cfg(feature = "cuda")]
use cudarc::driver::{CudaSlice, DevicePtr, DevicePtrMut, DeviceSlice};
use std::borrow::BorrowMut;

#[derive(Debug)]
pub struct DevSlice {
    device: Device,
    pub buffer: DevSliceEnum,
}

impl Drop for DevSlice {
    fn drop(&mut self) {
        if self.len() == 0 {
            return;
        }
        let device = self.device.clone();
        device.recycle(self.len(), self);
    }
}

#[derive(Debug)]
pub enum DevSliceEnum {
    CpuDevSlice(Vec<f32>),
    #[cfg(feature = "cuda")]
    CudaDevSlice(CudaSlice<f32>),
}

pub trait DevSliceTrait {
    fn as_ptr(&self) -> *const f32;
    fn as_mut_ptr(&mut self) -> *mut f32;
    fn get_values(&self) -> Result<Vec<f32>, Error>;
    fn set_values(&mut self, new_values: Vec<f32>) -> Result<(), Error>;
    fn len(&self) -> usize;
}

impl DevSlice {
    pub fn new(device: &Device, len: usize) -> DevSlice {
        // TODO remove unwrap
        let slice = device.slice(len as i32).unwrap();
        DevSlice {
            device: device.clone(),
            buffer: slice,
        }
    }
}

impl DevSliceTrait for DevSlice {
    fn as_ptr(&self) -> *const f32 {
        match &self.buffer {
            DevSliceEnum::CpuDevSlice(ref values) => values.as_ptr(),
            #[cfg(feature = "cuda")]
            DevSliceEnum::CudaDevSlice(ref values) => *values.device_ptr() as *const _,
        }
    }

    fn as_mut_ptr(&mut self) -> *mut f32 {
        match self.buffer.borrow_mut() {
            DevSliceEnum::CpuDevSlice(ref mut values) => values.as_mut_ptr(),
            #[cfg(feature = "cuda")]
            DevSliceEnum::CudaDevSlice(ref mut values) => *values.device_ptr_mut() as *mut _,
        }
    }

    fn get_values(&self) -> Result<Vec<f32>, Error> {
        match self.buffer {
            DevSliceEnum::CpuDevSlice(ref values) => Ok(values.clone()),
            #[cfg(feature = "cuda")]
            DevSliceEnum::CudaDevSlice(ref buffer) => {
                let mut values = vec![0.0; buffer.len()];
                let dev = buffer.device();
                let result = dev.dtoh_sync_copy_into(buffer, &mut values);
                match result {
                    Ok(_) => Ok(values),
                    _ => Err(error!(ErrorEnum::UnsupportedOperation)),
                }
            }
        }
    }

    fn set_values(&mut self, new_values: Vec<f32>) -> Result<(), Error> {
        match self.buffer.borrow_mut() {
            DevSliceEnum::CpuDevSlice(ref mut values) => {
                values.clear();
                values.extend_from_slice(new_values.as_slice());
                Ok(())
            }
            #[cfg(feature = "cuda")]
            DevSliceEnum::CudaDevSlice(ref mut buffer) => {
                let dev = buffer.device();
                dev.htod_sync_copy_into(&new_values, buffer)
                    .map_err(|_| error!(ErrorEnum::UnsupportedOperation))
            }
        }
    }

    fn len(&self) -> usize {
        match &self.buffer {
            DevSliceEnum::CpuDevSlice(buffer) => buffer.len(),
            #[cfg(feature = "cuda")]
            DevSliceEnum::CudaDevSlice(buffer) => buffer.len(),
        }
    }
}
