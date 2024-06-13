use crate::{
    stream::DeviceStream,
    tensor::{Error, Tensor},
};

pub struct Sub {}

impl Sub {
    pub fn execute(
        inputs: &[&Tensor],
        outputs: &[&Tensor],
        _device_stream: &DeviceStream,
    ) -> Result<(), Error> {
        let input_0 = inputs[0];
        let input_1 = inputs[1];
        let output = outputs[0];
        Tensor::copy(input_0, output)?;
        Tensor::sub(input_1, output)
    }
}
