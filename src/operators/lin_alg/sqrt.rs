use crate::{tensor::Error, tensor::Tensor, DeviceInterface};

pub struct Sqrt {}

impl Sqrt {
    pub fn execute(inputs: &[&Tensor], outputs: &[&Tensor]) -> Result<(), Error> {
        let input = inputs[0];
        let output = outputs[0];
        let device = input.device();
        device.sqrt(input, output)
    }
}
