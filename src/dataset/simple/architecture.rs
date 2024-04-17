use crate::{
    Accelerator, DifferentiableModule, DifferentiableModuleConfig, EmbeddingConfig, Error, Forward,
    FullDifferentiableModuleConfig, LinearConfig, ReshapeConfig, SoftmaxConfig, Tape, Tensor,
};
use std::borrow::Borrow;
use std::{cell::RefCell, rc::Rc};

pub struct Architecture {
    accelerator: Rc<Accelerator>,
    tape: Rc<RefCell<Tape>>,
    embedding: DifferentiableModule,
    linear_0: DifferentiableModule,
    sigmoid_0: DifferentiableModule,
    reshape: DifferentiableModule,
    linear_1: DifferentiableModule,
    sigmoid_1: DifferentiableModule,
    linear_2: DifferentiableModule,
    softmax: DifferentiableModule,
}

impl Default for Architecture {
    fn default() -> Self {
        let accelerator = Rc::new(Accelerator::default());
        let tape = Rc::new(RefCell::new(Tape::default()));
        let configs = architecture();
        let mut iterator = configs.iter().peekable();
        Self {
            accelerator: accelerator.clone(),
            tape: tape.clone(),
            embedding: FullDifferentiableModuleConfig {
                accelerator: &accelerator,
                tape: &tape,
                config: iterator.next().unwrap(),
            }
            .borrow()
            .into(),
            linear_0: FullDifferentiableModuleConfig {
                accelerator: &accelerator,
                tape: &tape,
                config: iterator.next().unwrap(),
            }
            .borrow()
            .into(),
            sigmoid_0: FullDifferentiableModuleConfig {
                accelerator: &accelerator,
                tape: &tape,
                config: iterator.next().unwrap(),
            }
            .borrow()
            .into(),
            reshape: FullDifferentiableModuleConfig {
                accelerator: &accelerator,
                tape: &tape,
                config: iterator.next().unwrap(),
            }
            .borrow()
            .into(),
            linear_1: FullDifferentiableModuleConfig {
                accelerator: &accelerator,
                tape: &tape,
                config: iterator.next().unwrap(),
            }
            .borrow()
            .into(),
            sigmoid_1: FullDifferentiableModuleConfig {
                accelerator: &accelerator,
                tape: &tape,
                config: iterator.next().unwrap(),
            }
            .borrow()
            .into(),
            linear_2: FullDifferentiableModuleConfig {
                accelerator: &accelerator,
                tape: &tape,
                config: iterator.next().unwrap(),
            }
            .borrow()
            .into(),
            softmax: FullDifferentiableModuleConfig {
                accelerator: &accelerator,
                tape: &tape,
                config: iterator.next().unwrap(),
            }
            .borrow()
            .into(),
        }
    }
}

impl Forward for Architecture {
    fn forward(&mut self, layer_input: &Tensor) -> Result<Tensor, Error> {
        let embedding = self.embedding.forward(layer_input)?;
        let linear_0 = self.linear_0.forward(&embedding)?;
        let sigmoid_0 = self.sigmoid_0.forward(&linear_0)?;
        let reshape = self.reshape.forward(&sigmoid_0)?;
        let linear_1 = self.linear_1.forward(&reshape)?;
        let sigmoid_1 = self.sigmoid_1.forward(&linear_1)?;
        let linear_2 = self.linear_2.forward(&sigmoid_1)?;
        let softmax = self.softmax.forward(&linear_2)?;
        Ok(softmax)
    }

    fn accelerator(&self) -> Rc<Accelerator> {
        self.accelerator.clone()
    }

    fn tape(&self) -> Rc<RefCell<Tape>> {
        self.tape.clone()
    }
}

pub fn architecture() -> Vec<DifferentiableModuleConfig> {
    vec![
        DifferentiableModuleConfig::Embedding(EmbeddingConfig {
            num_embeddings: 16,
            embedding_dim: 32,
        }),
        DifferentiableModuleConfig::Linear(LinearConfig {
            weights_rows: 16,
            weights_cols: 32,
            bias_rows: 6,
        }),
        DifferentiableModuleConfig::Sigmoid(Default::default()),
        DifferentiableModuleConfig::Reshape(ReshapeConfig {
            input_rows: 6,
            input_cols: 16,
            output_rows: 1,
            output_cols: 6 * 16,
        }),
        DifferentiableModuleConfig::Linear(LinearConfig {
            weights_rows: 32,
            weights_cols: 6 * 16,
            bias_rows: 1,
        }),
        DifferentiableModuleConfig::Sigmoid(Default::default()),
        DifferentiableModuleConfig::Linear(LinearConfig {
            weights_rows: 16,
            weights_cols: 32,
            bias_rows: 1,
        }),
        DifferentiableModuleConfig::Softmax(SoftmaxConfig {
            using_cross_entropy_loss: true,
        }),
    ]
}