use std::mem::swap;
use std::{cell::RefCell, ops::Deref, rc::Rc};

use crate::{
    Accelerator, DeltaWorkingMemory, DifferentiableModuleEnum, LossFunction, LossFunctionType,
    Tape, Tensor, TrainWorkingMemory,
};

use crate::gradient::DifferentiableModuleTrait;

/// Back-propagation
pub fn back_propagation(
    x: &Tensor,
    y: &Tensor,
    working_memory: &mut TrainWorkingMemory,
    error_working_memory: &mut DeltaWorkingMemory,
    loss_function: &LossFunctionType,
    accelerator: &Accelerator,
    tape: &Rc<RefCell<Tape>>,
) {
    let next_layer_delta = &mut working_memory.next_layer_delta;
    let layer_delta = &mut working_memory.layer_delta;
    let layers_count = {
        let tape = tape.deref().borrow();
        tape.records.len()
    };

    for layer_index in (0..layers_count).into_iter().rev() {
        let layer_output = &mut working_memory.layer_output;
        {
            let tape = tape.deref().borrow();
            let tensor = tape.records[layer_index].output.deref();
            layer_output.assign(accelerator, tensor);
        }

        let is_last_layer = layer_index == layers_count - 1;

        let previous_activation_tensor = &mut working_memory.previous_activation_tensor;

        match layer_index {
            0 => {
                previous_activation_tensor.assign(accelerator, x);
            }
            _ => {
                let tape = tape.deref().borrow();
                let tensor = tape.records[layer_index - 1].output.deref();
                previous_activation_tensor.assign(accelerator, tensor);
            }
        };

        if is_last_layer {
            // For the output layer, the next layer delta is the loss.
            let op_result = loss_function.derive(&accelerator, y, &layer_output, next_layer_delta);
            op_result.expect("Ok");
        }

        {
            let next_layer: Option<Rc<RefCell<DifferentiableModuleEnum>>> = if is_last_layer {
                None
            } else {
                let next_layer_index = layer_index + 1;
                let tape = tape.deref().borrow();
                let module = tape.records[next_layer_index].module.clone();
                Some(module)
            };

            let tmp = &mut working_memory.tmp;
            let layer_input: &Tensor = previous_activation_tensor;
            let back_propagated_delta = &mut working_memory.back_propagated_delta;

            let is_last_layer = next_layer.is_none();
            match next_layer {
                None => {
                    // use the output of the loss function¸
                    back_propagated_delta.assign(accelerator, next_layer_delta);
                }
                Some(next_layer) => {
                    // Hidden layer
                    let next_layer = next_layer.deref();
                    next_layer.borrow().backward(
                        accelerator,
                        next_layer_delta,
                        back_propagated_delta,
                    );
                }
            }

            let tape = tape.deref().borrow();
            let layer: &DifferentiableModuleEnum =
                &tape.records[layer_index].module.deref().borrow();
            layer.get_layer_output_delta(
                accelerator,
                error_working_memory,
                layer_input,
                layer_output,
                back_propagated_delta,
                is_last_layer,
                tmp,
            );

            tmp.clip(-1.0, 1.0, layer_delta)
        }

        {
            let tape = tape.deref().borrow();
            let layer: &mut DifferentiableModuleEnum =
                &mut tape.records[layer_index].module.deref().borrow_mut();
            layer.compute_gradient(accelerator, previous_activation_tensor, layer_delta);
        }

        swap(next_layer_delta, layer_delta);
    }
}