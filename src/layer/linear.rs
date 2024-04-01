use std::mem::swap;

use rand::{distributions::Uniform, thread_rng, Rng};

use crate::{
    ActivationFunction, DeltaWorkingMemory, Error, Layer, Tensor, TRANSPOSE_LHS, TRANSPOSE_RESULT,
    TRANSPOSE_RHS,
};

pub struct Linear {
    weights: Tensor,
    activation_function: Box<dyn ActivationFunction>,
    matrix_product: Tensor,
    weight_delta: Tensor,
    has_pending_change: bool,
    previous_a_time_output_delta: Tensor,
    addition: Tensor,
}

impl Linear {
    pub fn new(rows: usize, cols: usize, activation: Box<dyn ActivationFunction>) -> Self {
        let mut rng = thread_rng();
        let mut weights = Vec::new();
        let right = (6.0 as f32).sqrt() / (cols as f32 + rows as f32).sqrt();
        let left = -right;
        // Xavier Initialization, or Glorot Initialization,
        let uniform = Uniform::new(left, right);
        weights.resize(rows * cols, 0.0);
        for index in 0..weights.len() {
            weights[index] = rng.sample(uniform);
        }
        let weights = Tensor::new(rows, cols, weights);
        Linear {
            weights,
            activation_function: activation,
            matrix_product: Default::default(),
            weight_delta: Default::default(),
            has_pending_change: false,
            previous_a_time_output_delta: Default::default(),
            addition: Default::default(),
        }
    }
}

impl Layer for Linear {
    fn commit_change(&mut self) -> Result<(), Error> {
        if !self.has_pending_change {
            return Ok(());
        }
        let addition = &mut self.addition;
        {
            let weight_delta = &self.weight_delta;
            let weights = &self.weights;
            let op_result = weights.sub(weight_delta, addition);
            op_result.expect("Ok");
        }

        let weights = &mut self.weights;
        swap(weights, addition);
        self.has_pending_change = false;
        Ok(())
    }

    fn forward(&mut self, input: &Tensor, activation_tensor: &mut Tensor) -> Result<(), Error> {
        // Use the same convention that is used in tensorflow:
        // y = x @ W^T+b
        // Weights is on the right.
        // W is transposed.
        // X is not transposed.
        let weights = &self.weights;
        let matrix_product = &mut self.matrix_product;
        let op_result = Tensor::matmul(input, weights, matrix_product, TRANSPOSE_RHS);
        match op_result {
            Ok(_) => (),
            Err(_) => {
                let mut w_t = Tensor::default();
                weights.transpose(&mut w_t);
                println!("Incompatible shapes in matrix multiplication");
                println!("Between X {:?} and W^T {:?}", input.shape(), w_t.shape(),);
                debug_assert!(false);
            }
        }
        let activation_function = &self.activation_function;
        let op_result = activation_function.activate(&matrix_product, activation_tensor);
        op_result.expect("Ok");
        Ok(())
    }

    fn backward(&self, layer_delta: &Tensor, output_diff: &mut Tensor) {
        let layer_weights = &self.weights;

        let op_result = Tensor::matmul(
            layer_weights,
            layer_delta,
            output_diff,
            TRANSPOSE_LHS | TRANSPOSE_RHS | TRANSPOSE_RESULT,
        );

        op_result.expect("Ok");
    }

    fn get_layer_delta(
        &self,
        working_memory: &mut DeltaWorkingMemory,
        layer_activation_tensor: &Tensor,
        next_layer: Option<&Box<dyn Layer>>,
        next_layer_delta: &Tensor,
        using_softmax_and_cross_entropy_loss: bool,
        layer_delta: &mut Tensor,
    ) {
        let layer_f_derivative = &mut working_memory.layer_f_derivative;
        let layer_activation_function = &self.activation_function;
        let output_diff = &mut working_memory.output_diff;

        match next_layer {
            None => {
                // use the output of the loss function.
                output_diff.assign(next_layer_delta);
            }
            Some(next_layer) => {
                // Hidden layer
                next_layer.backward(next_layer_delta, output_diff);
            }
        }

        // Compute activation function derivative.
        let is_last_layer = next_layer.is_none();
        if is_last_layer && using_softmax_and_cross_entropy_loss {
            layer_activation_tensor
                .scalar_add(1.0, layer_f_derivative)
                .expect("Ok");
        } else {
            let matrix_product = &self.matrix_product;
            let op_result = layer_activation_function.derive(
                matrix_product,
                layer_activation_tensor,
                layer_f_derivative,
            );
            op_result.expect("Ok");
        }

        let op_result = layer_f_derivative.element_wise_mul(output_diff, layer_delta);
        op_result.expect("Ok");
    }

    fn plan_change(
        &mut self,
        learning_rate: f32,
        previous_activation: &Tensor,
        layer_delta: &Tensor,
    ) {
        let previous_a_time_output_delta = &mut self.previous_a_time_output_delta;
        let op_result = Tensor::matmul(
            previous_activation,
            layer_delta,
            previous_a_time_output_delta,
            TRANSPOSE_LHS | TRANSPOSE_RESULT,
        );
        op_result.expect("Ok");

        let weight_delta = &mut self.weight_delta;
        let op_result = previous_a_time_output_delta.scalar_mul(learning_rate, weight_delta);
        op_result.expect("Ok");
        self.has_pending_change = true;
    }
}
