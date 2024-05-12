use crate::devices::Device;
use crate::{ActivationFunction, Operator, TensorF32, UnaryOperator};
use crate::{Error, Tensor};
use std::f32::consts::E;
use std::ops::Deref;
use std::rc::Rc;

/// https://onnx.ai/onnx/operators/onnx__Softmax.html
#[derive(Clone)]
pub struct Softmax {
    device: Device,
    next_op_is_cross_entropy_loss: bool,
}

impl Softmax {
    pub fn new(device: &Device, next_op_is_cross_entropy_loss: bool) -> Self {
        Self {
            device: device.clone(),
            next_op_is_cross_entropy_loss,
        }
    }
}

impl ActivationFunction for Softmax {
    fn activate(product_matrix: &TensorF32, result: &TensorF32) -> Result<(), Error> {
        let rows = product_matrix.rows();
        let cols = product_matrix.cols();
        let values = product_matrix.get_values()?;
        let mut result_values = result.get_values()?;
        let mut row = 0;
        while row < rows {
            // Find max

            let mut max = values[product_matrix.index(row, 0)];
            let mut col = 0;
            while col < cols {
                let x = values[product_matrix.index(row, col)];
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
                let x = values[product_matrix.index(row, col)];
                let y = E.powf(x - max);
                result_values[result.index(row, col)] = y;
                sum += y;
                col += 1;
            }

            // Divide every value by sum.

            let mut col = 0;
            while col < cols {
                let x = result_values[result.index(row, col)];
                let y = x / sum;
                result_values[result.index(row, col)] = y;
                col += 1;
            }
            row += 1;
        }
        result.set_values(result_values);
        Ok(())
    }

    fn derive(
        _product_matrix: &TensorF32,
        activation_matrix: &TensorF32,
        result: &mut TensorF32,
    ) -> Result<(), Error> {
        let rows = activation_matrix.rows();
        let cols = activation_matrix.cols();
        let values = activation_matrix.get_values()?;
        let mut result_values = result.get_values()?;
        let mut row = 0;
        while row < rows {
            let mut col = 0;
            while col < cols {
                let x = values[activation_matrix.index(row, col)];
                let y = x * (1.0 - x);
                result_values[result.index(row, col)] = y;
                col += 1;
            }
            row += 1;
        }
        result.set_values(result_values);

        Ok(())
    }
}

impl UnaryOperator for Softmax {
    fn forward(&self, input: &Tensor) -> Result<Tensor, Error> {
        let input_t: &TensorF32 = &input.tensor().deref().borrow();
        let rows = input_t.rows();
        let cols = input_t.cols();
        let len = rows * cols;
        let output = self.device.tensor(rows, cols, vec![0.0; len], true, false);
        let inputs = &[input];
        let outputs = &[&output];
        output.push_forward_instruction(Rc::new(self.clone()), inputs, outputs);
        output.push_backward_instruction(
            Rc::new(SoftmaxBackward::new(
                &self.device,
                self.next_op_is_cross_entropy_loss,
            )),
            outputs,
            inputs,
        );
        Ok(output)
    }
}

impl Operator for Softmax {
    fn name(&self) -> &str {
        "Softmax"
    }

    fn forward(&self, inputs: &[&Tensor], outputs: &[&Tensor]) -> Result<(), Error> {
        let input = inputs[0].tensor().deref().borrow();
        let output = outputs[0].tensor().deref().borrow();
        Self::activate(&input, &output)
    }
}

pub struct SoftmaxBackward {
    device: Device,
    next_op_is_cross_entropy_loss: bool,
}

impl SoftmaxBackward {
    pub fn new(device: &Device, next_op_is_cross_entropy_loss: bool) -> Self {
        Self {
            device: device.clone(),
            next_op_is_cross_entropy_loss,
        }
    }
}

impl Operator for SoftmaxBackward {
    fn name(&self) -> &str {
        "SoftmaxBackward"
    }

    fn forward(&self, inputs: &[&Tensor], outputs: &[&Tensor]) -> Result<(), Error> {
        if outputs[0].requires_grad() {
            let output_gradient: &mut TensorF32 = &mut outputs[0].gradient().deref().borrow_mut();
            let input_gradient: &TensorF32 = &inputs[0].gradient().deref().borrow();
            // Compute activation function derivative.
            if self.next_op_is_cross_entropy_loss {
                // Softmax and Cross Entropy Loss are best friends.
                return TensorF32::copy(input_gradient, output_gradient);
            }

            let output: &TensorF32 = &outputs[0].tensor().deref().borrow();
            let input: &TensorF32 = &inputs[0].tensor().deref().borrow();
            let rows = output.rows();
            let cols = output.cols();
            let len = rows * cols;
            let mut layer_f_derivative = self.device.tensor_f32(rows, cols, vec![0.0; len]);
            Softmax::derive(output, input, &mut layer_f_derivative)?;
            TensorF32::mul(&layer_f_derivative, input_gradient, output_gradient)?;
        }

        Ok(())
    }
}
