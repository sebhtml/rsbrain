use std::{cell::RefCell, ops::Deref, rc::Rc};

use crate::{devices::Device, DeltaWorkingMemory, Error, LearningTensor, OperatorTrait, Tensor};

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
    fn backward(
        &self,
        device: &Device,
        _error_working_memory: &mut DeltaWorkingMemory,
        _inputs: &Vec<Rc<RefCell<Tensor>>>,
        _output: &Rc<RefCell<Tensor>>,
        back_propagated_delta: &Rc<RefCell<Tensor>>,
    ) -> Result<(Rc<RefCell<Tensor>>, Vec<LearningTensor>), Error> {
        let backward_gradient = Rc::new(RefCell::new(device.tensor(0, 0, vec![])));
        let back_propagated_delta: &Tensor = &back_propagated_delta.deref().borrow();
        {
            let backward_gradient: &mut Tensor = &mut backward_gradient.deref().borrow_mut();
            backward_gradient.assign(device, back_propagated_delta);
            backward_gradient.reshape(self.input_rows, self.input_cols)?;
        }

        Ok((backward_gradient, vec![]))
    }

    fn forward(
        &self,
        device: &Device,
        inputs: &Vec<Rc<RefCell<Tensor>>>,
    ) -> Result<Rc<RefCell<Tensor>>, Error> {
        debug_assert_eq!(inputs.len(), 1);
        let input: &Tensor = &inputs[0].deref().borrow();
        debug_assert_eq!(input.rows(), self.input_rows);
        debug_assert_eq!(input.cols(), self.input_cols);
        let output = Rc::new(RefCell::new(device.tensor(0, 0, vec![])));
        {
            let output: &mut Tensor = &mut output.deref().borrow_mut();
            output.assign(device, input);
            output.reshape(self.output_rows, self.output_cols)?;
        }
        Ok(output)
    }

    fn name(&self) -> &str {
        "Reshape"
    }
}
