use std::{cell::RefCell, ops::Deref, rc::Rc};

use crate::Tensor;

pub struct Gradient {
    tensor: Rc<RefCell<Tensor>>,
    gradient: Rc<RefCell<Tensor>>,
}

impl Gradient {
    pub fn new(tensor: Rc<RefCell<Tensor>>, gradient: Rc<RefCell<Tensor>>) -> Self {
        debug_assert_eq!(
            gradient.deref().borrow().shape(),
            tensor.deref().borrow().shape()
        );
        Self { tensor, gradient }
    }
    pub fn tensor(&self) -> &Rc<RefCell<Tensor>> {
        &self.tensor
    }
    pub fn gradient(&self) -> &Rc<RefCell<Tensor>> {
        &self.gradient
    }
}
