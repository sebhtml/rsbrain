#[cfg(test)]
pub mod tests;
mod train;
use std::{cell::RefCell, ops::Deref, rc::Rc};
pub use train::*;

use crate::{
    accelerator::Accelerator, back_propagation, Error, Forward, Operator, Optimizer,
    OptimizerTrait, Tape, Tensor,
};

pub struct Network {
    architecture: Box<dyn Forward>,
    loss_function: Operator,
    accelerator: Rc<Accelerator>,
    optimizer: Optimizer,
    tape: Rc<RefCell<Tape>>,
}

pub struct TrainWorkingMemory {
    pub layer_output: Tensor,
    pub next_layer_delta: Tensor,
    pub back_propagated_delta: Tensor,
    pub layer_delta: Tensor,
    pub previous_activation_tensor: Tensor,
    pub tmp: Tensor,
}

impl Default for TrainWorkingMemory {
    fn default() -> Self {
        Self {
            layer_output: Default::default(),
            next_layer_delta: Default::default(),
            back_propagated_delta: Default::default(),
            layer_delta: Default::default(),
            previous_activation_tensor: Default::default(),
            tmp: Default::default(),
        }
    }
}

pub struct DeltaWorkingMemory {
    pub layer_f_derivative: Tensor,
}

impl Default for DeltaWorkingMemory {
    fn default() -> Self {
        Self {
            layer_f_derivative: Default::default(),
        }
    }
}

pub struct PredictWorkingMemory {
    pub previous_activation_tensor: Tensor,
    pub activation_tensor: Tensor,
    pub activation_tensors: Vec<Tensor>,
}

impl PredictWorkingMemory {
    pub fn new(examples_count: usize) -> Self {
        Self {
            previous_activation_tensor: Default::default(),
            activation_tensor: Default::default(),
            activation_tensors: vec![Tensor::default(); examples_count],
        }
    }
}

impl Network {
    pub fn new(architecture: Box<dyn Forward>, loss_function: Operator) -> Self {
        let accelerator = architecture.accelerator();
        let tape = architecture.tape();
        Self {
            architecture,
            loss_function,
            accelerator,
            tape,
            optimizer: Default::default(),
        }
    }

    pub fn train(
        &mut self,
        working_memory: &mut TrainWorkingMemory,
        error_working_memory: &mut DeltaWorkingMemory,
        epoch: usize,
        inputs: &Vec<Rc<Tensor>>,
        outputs: &Vec<Rc<Tensor>>,
    ) -> Result<(), Error> {
        for i in 0..inputs.len() {
            self.train_back_propagation(
                working_memory,
                error_working_memory,
                epoch,
                i,
                &inputs[i],
                &outputs[i],
            )?;
        }
        Ok(())
    }

    pub fn total_error(
        &mut self,
        inputs: &Vec<Rc<Tensor>>,
        outputs: &Vec<Rc<Tensor>>,
    ) -> Result<f32, Error> {
        let mut total_error = 0.0;
        for i in 0..inputs.len() {
            let output = self.forward(&inputs[i])?;
            let target = &outputs[i];
            let example_error = self
                .loss_function
                .forward_inputs(&vec![target.clone(), output.clone()])
                .expect("Ok");
            let example_error: &Tensor = example_error.deref();
            let example_error: f32 = example_error.try_into()?;
            total_error += example_error;
        }

        Ok(total_error)
    }

    fn train_back_propagation(
        &mut self,
        working_memory: &mut TrainWorkingMemory,
        error_working_memory: &mut DeltaWorkingMemory,
        _epoch: usize,
        _example_index: usize,
        x: &Rc<Tensor>,
        y: &Rc<Tensor>,
    ) -> Result<(), Error> {
        self.tape.deref().borrow_mut().clear();

        let output = self.forward(x)?;

        self.loss_function
            .forward_inputs(&vec![y.clone(), output.clone()])?;

        let gradients = back_propagation(
            working_memory,
            error_working_memory,
            &self.accelerator,
            &self.tape,
        )?;

        self.optimizer.optimize(gradients, &self.accelerator);

        Ok(())
    }

    pub fn predict_many(&mut self, inputs: &Vec<Rc<Tensor>>) -> Result<Vec<Rc<Tensor>>, Error> {
        let len = inputs.len();
        let mut outputs = vec![];
        let mut i = 0;
        while i < len {
            let input = &inputs[i];
            let output = self.forward(input)?;
            outputs.push(output);
            i += 1;
        }
        Ok(outputs)
    }

    pub fn forward(&mut self, input: &Rc<Tensor>) -> Result<Rc<Tensor>, Error> {
        self.architecture.forward(input)
    }
}
