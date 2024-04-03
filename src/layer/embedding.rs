use rand::{distributions::Uniform, thread_rng, Rng};

use crate::{DeltaWorkingMemory, Error, Layer, LayerType, Tensor, TensorTrait};

pub struct Embedding {
    embedding_table: Tensor,
    activation_tensor: Tensor,
}

impl Embedding {
    pub fn new(_hidden_dimensions: usize) -> Self {
        // TODO
        Self {
            embedding_table: get_u8_embedding_table(),
            activation_tensor: Default::default(),
        }
    }
}

impl Layer for Embedding {
    fn plan_change(
        &mut self,
        _learning_rate: f32,
        _previous_activation: &Tensor,
        _layer_delta: &Tensor,
    ) {
        // TODO
    }

    fn commit_change(&mut self) -> Result<(), Error> {
        // TODO
        Ok(())
    }

    fn forward(&mut self, input: &Tensor) -> Result<(), Error> {
        // TODO
        let x = add_embeddings(&self.embedding_table, input);
        self.activation_tensor.assign(&x);
        Ok(())
    }

    fn get_activation_tensor<'a>(&'a self) -> &'a Tensor {
        &self.activation_tensor
    }

    fn backward(&self, _layer_delta: &Tensor, _output_diff: &mut Tensor) {
        panic!("Embedding can not go backward !");
    }

    fn get_layer_delta(
        &self,
        _working_memory: &mut DeltaWorkingMemory,
        _next_layer: Option<&LayerType>,
        _next_layer_delta: &Tensor,
        _using_softmax_and_cross_entropy_loss: bool,
        layer_delta: &mut Tensor,
    ) {
        // TODO
        let new_rows = self.activation_tensor.rows();
        let new_cols = self.activation_tensor.cols();
        layer_delta.reshape(new_rows, new_cols);
    }
}

pub struct EmbeddingConfig {
    pub hidden_dimensions: usize,
}

impl Into<Embedding> for &EmbeddingConfig {
    fn into(self) -> Embedding {
        Embedding::new(self.hidden_dimensions)
    }
}

fn get_u8_embedding_table() -> Tensor {
    let mut rng = thread_rng();
    let mut embeddings_table: Vec<f32> = Vec::new();
    let left = 0.1;
    let right = 0.9;
    let number_of_different_tokens = 256;
    let width = 256;
    let uniform = Uniform::new(left, right);

    let mut token = 0;
    while token < number_of_different_tokens {
        let mut token_embeddings: Vec<f32> = Vec::new();
        for _ in 0..width {
            let value = rng.sample(uniform);
            token_embeddings.push(value);
        }
        embeddings_table.append(&mut token_embeddings);
        token += 1;
    }
    Tensor::new(width, width, embeddings_table)
}

// TODO use &mut output argument for result
fn add_embeddings(embedding_table: &Tensor, input: &Tensor) -> Tensor {
    let mut values = vec![];
    let mut row = 0;
    let input: &Vec<usize> = input.into();
    let rows = input.len();
    let mut row_embeddings = Tensor::default();
    while row < rows {
        let index = input[row];
        embedding_table.row(index, &mut row_embeddings);
        let row_embeddings: &Vec<f32> = (&row_embeddings).into();
        values.append(&mut row_embeddings.clone());
        row += 1;
    }
    Tensor::new(input.len(), embedding_table.cols(), values)
}
