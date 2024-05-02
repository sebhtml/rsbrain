use std::{cell::RefCell, ops::Deref, rc::Rc};

use crate::{Device, Error, OperatorTrait, Record, Tape, TensorF32};

#[derive(Clone)]
pub struct Tensor {
    tensor: Rc<RefCell<TensorF32>>,
    gradient: Rc<RefCell<TensorF32>>,
}

impl Tensor {
    pub fn new(tensor: Rc<RefCell<TensorF32>>, gradient: Rc<RefCell<TensorF32>>) -> Self {
        Self { tensor, gradient }
    }
    pub fn tensor(&self) -> &Rc<RefCell<TensorF32>> {
        &self.tensor
    }
    pub fn gradient(&self) -> &Rc<RefCell<TensorF32>> {
        &self.gradient
    }

    /// Back-propagation
    pub fn backward(
        &self,
        device: &Device,
        tape: &Rc<RefCell<Tape>>,
    ) -> Result<Vec<Tensor>, Error> {
        let tape: &Tape = &tape.deref().borrow();
        let records: &Vec<Record> = &tape.records();

        for record in records.iter().rev() {
            let operator: &Box<dyn OperatorTrait> = &record.operator().deref().borrow();
            let inputs = record.inputs();
            let output = record.output();

            // Store enabled gradients to optimize them later.
            operator.backward(device, inputs, output)?;

            // Clip the backward gradients.
            for input in inputs {
                let backward_gradient: &mut TensorF32 = &mut input.gradient().deref().borrow_mut();
                let back_propagated_gradient = device.tensor_f32(
                    backward_gradient.rows(),
                    backward_gradient.cols(),
                    backward_gradient.get_values()?,
                );
                back_propagated_gradient.clip(-1.0, 1.0, backward_gradient)?;
            }
        }
        Ok(device.tensors_with_requires_grad())
    }
}
