use crate::{
    gradient_instruction, inference_instruction, new_tensor, new_tensor_with_grad,
    stream::DeviceStream,
    tensor::{Error, Tensor},
    Device, ExecutableOperator, NaryOperator, OpCode, OperatorAttributes, TensorWithGrad,
};

#[cfg(test)]
mod tests;

pub struct Concat {
    device: Device,
}

impl Concat {
    pub fn new(device: &Device) -> Self {
        Self {
            device: device.clone(),
        }
    }
}

impl ExecutableOperator for Concat {
    fn execute(
        _attributes: &OperatorAttributes,
        inputs: &[&Tensor],
        outputs: &[&Tensor],
        device_stream: &DeviceStream,
    ) -> Result<(), Error> {
        let dst = outputs[0];
        for input_index in 0..inputs.len() {
            let src = inputs[input_index];
            let src_col = 0;
            let input_rows = src.rows();
            let input_cols = src.cols();
            for src_row in 0..input_rows {
                let dst_row = src_row;
                let dst_col = input_index * input_cols;
                Tensor::copy_slice(
                    src.cols(),
                    &src,
                    src_row,
                    src_col,
                    &dst,
                    dst_row,
                    dst_col,
                    device_stream,
                )?;
            }
        }
        Ok(())
    }
}

impl NaryOperator for Concat {
    fn forward(&self, inputs_n: &[&TensorWithGrad]) -> Result<TensorWithGrad, Error> {
        let rows = inputs_n[0].tensor().rows();
        let cols = inputs_n[0].tensor().cols();
        for input in inputs_n.iter() {
            debug_assert_eq!(input.tensor().rows(), rows);
            debug_assert_eq!(input.tensor().cols(), cols);
        }
        let cols = inputs_n.len() * cols;
        let len = rows * cols;
        let values = vec![0.0; len];
        let output = new_tensor_with_grad!(self.device, rows, cols, values, inputs_n, true, false)?;
        let inputs = inputs_n;
        let outputs = [&output];
        let inputs: Vec<Tensor> = inputs.iter().map(|t| t.tensor().clone()).collect();
        let zero = new_tensor!(self.device, 1, 1, vec![0.0])?;
        output.push_instruction(inference_instruction!(
            OpCode::ScalarMul,
            OperatorAttributes::None,
            &[&zero, &outputs[0].tensor()],
            &[&outputs[0].tensor()],
        ));
        output.push_instruction(inference_instruction!(
            OpCode::ScalarMul,
            OperatorAttributes::None,
            &[&zero, &outputs[0].gradient()],
            &[&outputs[0].gradient()],
        ));
        output.push_instruction(inference_instruction!(
            OpCode::Concat,
            OperatorAttributes::None,
            &inputs.iter().collect::<Vec<_>>(),
            &[&outputs[0].tensor()],
        ));
        let inputs = [&output];
        let outputs = inputs_n;
        let outputs: Vec<Tensor> = outputs.iter().map(|t| t.gradient().clone()).collect();
        output.push_instruction(gradient_instruction!(
            OpCode::Unconcat,
            OperatorAttributes::None,
            &[&inputs[0].gradient()],
            &outputs.iter().collect::<Vec<_>>(),
        ));
        Ok(output)
    }
}

pub struct Unconcat {}

impl ExecutableOperator for Unconcat {
    fn execute(
        _attributes: &OperatorAttributes,
        inputs: &[&Tensor],
        outputs: &[&Tensor],
        device_stream: &DeviceStream,
    ) -> Result<(), Error> {
        let src = inputs[0];
        for output_index in 0..outputs.len() {
            let dst = outputs[output_index];
            let dst_col = 0;
            let input_rows = dst.rows();
            let input_cols = dst.cols();
            for dst_row in 0..input_rows {
                let src_row = dst_row;
                let src_col = output_index * input_cols;
                Tensor::copy_slice(
                    dst.cols(),
                    src,
                    src_row,
                    src_col,
                    dst,
                    dst_row,
                    dst_col,
                    device_stream,
                )?;
            }
        }
        Ok(())
    }
}
