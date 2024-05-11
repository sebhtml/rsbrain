mod activation;
pub use activation::*;
mod view;
pub use view::*;
mod loss;
pub use loss::*;
mod lin_alg;
pub use lin_alg::*;
mod attention;
pub use attention::*;

use crate::{Error, Tensor};
use core::fmt::Debug;

pub trait UnaryOperator {
    fn forward(&self, input: &Tensor) -> Result<Tensor, Error>;
}

pub trait BinaryOperator {
    fn forward(&self, input_1: &Tensor, input_2: &Tensor) -> Result<Tensor, Error>;
}

pub trait TernaryOperator {
    fn forward(
        &self,
        input_1: &Tensor,
        input_2: &Tensor,
        input_3: &Tensor,
    ) -> Result<Tensor, Error>;
}

pub trait Operator {
    fn name(&self) -> &str;
    fn forward_realize(&self, inputs: &[&Tensor], output: &Tensor) -> Result<(), Error>;
    fn backward(&self, inputs: &[&Tensor], output: &Tensor) -> Result<(), Error>;
}

impl Debug for dyn Operator {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}
