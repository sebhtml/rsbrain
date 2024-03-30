use std::{cell::RefCell, rc::Rc};

use rand::{distributions::Uniform, thread_rng, Rng};

use crate::{ActivationFunction, Error, Layer, Tensor};

pub struct Linear {
    weights: Rc<RefCell<Tensor>>,
    activation: Rc<dyn ActivationFunction>,
}

impl Linear {
    pub fn new(rows: usize, cols: usize, activation: Rc<dyn ActivationFunction>) -> Self {
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
            weights: Rc::new(RefCell::new(weights)),
            activation: activation,
        }
    }
}
impl Layer for Linear {
    fn weights(&self) -> Rc<RefCell<Tensor>> {
        self.weights.clone()
    }

    fn activation(&self) -> Rc<dyn ActivationFunction> {
        self.activation.clone()
    }

    fn forward(&self, input: &Tensor, w_t: &mut Tensor, result: &mut Tensor) -> Result<(), Error> {
        self.weights.borrow().transpose(w_t);
        let op_result = input.matmul(w_t, result);
        match op_result {
            Ok(_) => (),
            Err(_) => {
                println!("Incompatible shapes in matrix multiplication");
                println!("Between X {:?} and W^T {:?}", input.shape(), w_t.shape(),);
                debug_assert!(false);
            }
        }
        Ok(())
    }
}
