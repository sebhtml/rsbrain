#[cfg(test)]
pub mod tests;
mod train;
use std::{cell::RefCell, ops::Deref, rc::Rc};
pub use train::*;

use crate::{
    devices::Device, Error, Forward, Operator, Optimizer, OptimizerTrait, Tape, Tensor, TensorF32,
};

pub struct Network {
    architecture: Box<dyn Forward>,
    loss_function: Operator,
    device: Rc<Device>,
    optimizer: Optimizer,
    tape: Rc<RefCell<Tape>>,
}

impl Network {
    pub fn new(architecture: Box<dyn Forward>, loss_function: Operator) -> Self {
        let device = architecture.device();
        let tape = architecture.tape();
        Self {
            architecture,
            loss_function,
            device,
            tape,
            optimizer: Default::default(),
        }
    }

    pub fn train(
        &mut self,
        learning_rate: f32,
        epoch: usize,
        inputs: &Vec<Tensor>,
        outputs: &Vec<Tensor>,
    ) -> Result<(), Error> {
        for i in 0..inputs.len() {
            self.train_back_propagation(learning_rate, epoch, i, &inputs[i], &outputs[i])?;
        }
        Ok(())
    }

    pub fn example_loss(
        &self,
        actual_output: &Tensor,
        expected_output: &Tensor,
    ) -> Result<f32, Error> {
        let example_loss = self
            .loss_function
            .forward(&[expected_output.clone(), actual_output.clone()])?;
        let example_loss: &TensorF32 = &example_loss.tensor().deref().borrow();
        let example_loss: f32 = example_loss.try_into()?;
        Ok(example_loss)
    }

    pub fn total_loss(&self, inputs: &[Tensor], outputs: &[Tensor]) -> Result<f32, Error> {
        let mut total_error = 0.0;
        for i in 0..inputs.len() {
            let output = self.forward(&[inputs[i].clone()])?;
            let expected_output = &outputs[i];
            let example_error = self.example_loss(&output, expected_output)?;
            total_error += example_error;
        }

        Ok(total_error)
    }

    fn train_back_propagation(
        &mut self,
        learning_rate: f32,
        _epoch: usize,
        _example_index: usize,
        x: &Tensor,
        y: &Tensor,
    ) -> Result<(), Error> {
        self.tape.deref().borrow_mut().clear();

        let output = self.forward(&[x.clone()])?;

        let loss = self.loss_function.forward(&[y.clone(), output.clone()])?;

        let gradients = loss.backward(&self.device, &self.tape)?;

        self.optimizer
            .optimize(&gradients, &self.device, learning_rate)?;

        for gradient in gradients {
            let gradient: &mut TensorF32 = &mut gradient.gradient().deref().borrow_mut();
            TensorF32::scalar_mul(&self.device, 0.0, gradient)?;
        }
        Ok(())
    }
}

impl Forward for Network {
    fn forward(&self, inputs: &[Tensor]) -> Result<Tensor, Error> {
        self.architecture.forward(inputs)
    }

    fn device(&self) -> Rc<Device> {
        self.device.clone()
    }

    fn tape(&self) -> Rc<RefCell<Tape>> {
        self.tape.clone()
    }
}
