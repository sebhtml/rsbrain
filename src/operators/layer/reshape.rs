use std::ops::Deref;

use crate::{devices::Device, Error, OperatorTrait, Tensor, TensorF32};

pub struct Reshape {
    input_rows: usize,
    input_cols: usize,
    output_rows: usize,
    output_cols: usize,
}

impl Reshape {
    pub fn new(
        input_rows: usize,
        input_cols: usize,
        output_rows: usize,
        output_cols: usize,
    ) -> Self {
        Self {
            input_rows,
            input_cols,
            output_rows,
            output_cols,
        }
    }
}

impl OperatorTrait for Reshape {
    fn backward(&self, device: &Device, inputs: &[Tensor], output: &Tensor) -> Result<(), Error> {
        let output_gradient: &TensorF32 = &output.gradient().deref().borrow();
        let backward_gradient: &mut TensorF32 = &mut inputs[0].gradient().deref().borrow_mut();
        TensorF32::copy(device, output_gradient, backward_gradient)?;
        backward_gradient.reshape(self.input_rows, self.input_cols)?;
        Ok(())
    }

    fn forward(&self, device: &Device, inputs: &[Tensor]) -> Result<Tensor, Error> {
        debug_assert_eq!(inputs.len(), 1);
        let input: &TensorF32 = &inputs[0].tensor().deref().borrow();
        debug_assert_eq!(input.rows(), self.input_rows);
        debug_assert_eq!(input.cols(), self.input_cols);
        let rows = input.rows();
        let cols = input.cols();
        let len = rows * cols;
        let output = device.tensor(inputs, rows, cols, vec![0.0; len], false);
        {
            let output: &mut TensorF32 = &mut output.tensor().deref().borrow_mut();
            TensorF32::copy(device, input, output)?;
            output.reshape(self.output_rows, self.output_cols)?;
        }
        Ok(output)
    }

    fn name(&self) -> &str {
        "Reshape"
    }
}
