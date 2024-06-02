use crate::{
    devices::Device, gradient_instruction, inference_instruction, tensor::Error, tensor::Tensor,
    OpCode, TensorWithGrad, UnaryOperator,
};

pub struct Reshape {
    device: Device,
    input_size: Vec<usize>,
    output_size: Vec<usize>,
}

impl Reshape {
    pub fn new(device: &Device, input_size: Vec<usize>, output_size: Vec<usize>) -> Self {
        Self {
            device: device.clone(),
            input_size,
            output_size,
        }
    }

    pub fn execute(
        output_size: &[usize],
        inputs: &[&Tensor],
        outputs: &[&Tensor],
    ) -> Result<(), Error> {
        let input = inputs[0];
        let output = outputs[0];
        Tensor::copy(input, output)?;
        output.resize(output_size)
    }
}

impl UnaryOperator for Reshape {
    fn forward(&self, input: &TensorWithGrad) -> Result<TensorWithGrad, Error> {
        let input_tensor: &Tensor = &input.tensor();
        debug_assert_eq!(*input_tensor.size(), self.input_size);
        let rows = self.output_size[0];
        let cols = self.output_size[1];
        let len = rows * cols;
        let output = self
            .device
            .tensor_with_grad(rows, cols, vec![0.0; len], &[input], true, false)
            .unwrap();
        let inputs = [input];
        let outputs = [&output];
        let zero = self.device.tensor(1, 1, vec![0.0]).unwrap();
        output.push_instruction(inference_instruction!(
            OpCode::ScalarMul,
            &[&zero, &outputs[0].tensor()],
            &[&outputs[0].tensor()],
        ));
        output.push_instruction(inference_instruction!(
            OpCode::ScalarMul,
            &[&zero, &outputs[0].gradient()],
            &[&outputs[0].gradient()],
        ));
        output.push_instruction(inference_instruction!(
            OpCode::Reshape(self.output_size.clone()),
            &[&inputs[0].tensor()],
            &[&outputs[0].tensor()],
        ));
        let inputs = [&output];
        let outputs = [input];

        if outputs[0].gradient().requires_grad() {
            output.push_instruction(gradient_instruction!(
                OpCode::Reshape(self.input_size.clone()),
                &[&inputs[0].gradient()],
                &[&outputs[0].gradient()],
            ));
        }

        Ok(output)
    }
}
