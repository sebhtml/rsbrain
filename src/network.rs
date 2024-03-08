use rand::Rng;

use crate::{activation::sigmoid, sigmoid_derivative, Matrix};
pub struct Network {
    layers: Vec<Matrix>,
}

impl Network {
    pub fn new() -> Self {
        //let layer_sizes = vec![(16, 4), (1, 16)];
        //let layer_sizes = vec![(16, 4), (32, 16), (16, 32), (1, 16)];
        let layer_sizes = vec![(1, 4)];
        Self {
            layers: layer_sizes
                .iter()
                .map(|(rows, cols)| -> Matrix {
                    let mut weights = Vec::new();
                    weights.resize(rows * cols, 0.0);
                    for index in 0..weights.len() {
                        weights[index] = rand::thread_rng().gen_range(0.0..1.0);
                    }
                    Matrix::new(*rows, *cols, weights)
                })
                .collect(),
        }
    }

    pub fn train(&mut self, inputs: &Vec<Vec<f32>>, outputs: &Vec<Vec<f32>>) {
        for i in 0..inputs.len() {
            self.train_back_propagation(i, &inputs[i], &outputs[i]);
        }
    }

    pub fn total_error(&self, inputs: &Vec<Vec<f32>>, outputs: &Vec<Vec<f32>>) -> f32 {
        let mut total_error = 0.0;
        for i in 0..inputs.len() {
            let predicted = self.predict(&inputs[i]);
            let target = &outputs[i];
            let example_error = self.compute_error(target, &predicted);
            println!(
                "Example Error example {} target {:?} predicted {:?} error {}",
                i, target, predicted, example_error
            );
            total_error += example_error;
        }

        total_error
    }

    // https://web.stanford.edu/group/pdplab/originalpdphandbook/Chapter%205.pdf
    fn train_back_propagation(&mut self, _example: usize, x: &Vec<f32>, y: &Vec<f32>) {
        let learning_rate = 0.5;
        println!("[train_with_one_example] x {:?} y {:?}", x, y);
        let mut matrix_products: Vec<Matrix> = Vec::new();
        let mut activations: Vec<Matrix> = Vec::new();
        let x = x.clone();
        // TODO add constant bias
        // Add a constant for bias
        //x.push(1.0);
        let x = Matrix::new(x.len(), 1, x);

        for (layer, layer_weights) in self.layers.iter().enumerate() {
            let previous_activation = {
                if layer == 0 {
                    &x
                } else {
                    &activations[activations.len() - 1]
                }
            };
            println!("Layer {} weights: {}", layer, layer_weights);
            println!("Inputs: {}", previous_activation);

            let matrix_product = layer_weights * previous_activation;

            match matrix_product {
                Ok(matrix_product) => {
                    matrix_products.push(matrix_product.clone());
                    let mut activation = matrix_product.clone();
                    for row in 0..activation.rows() {
                        for col in 0..activation.cols() {
                            activation.set(row, col, sigmoid(matrix_product.get(row, col)));
                        }
                    }
                    println!("matrix_product: {}", matrix_product);
                    println!("Activation: {}", activation);
                    activations.push(activation);
                }
                _ => {
                    println!("Incompatible shapes in matrix multiplication");
                    println!("Between  W {} and A {}", layer_weights, previous_activation,);
                }
            }
        }

        // Back-propagation
        let mut weight_deltas = self.layers.clone();
        let mut layer_diffs = Vec::new();
        layer_diffs.resize(self.layers.len(), Vec::<f32>::new());

        for (layer, _) in self.layers.iter().enumerate().rev() {
            let layer = layer.to_owned();
            let layer_weights = &self.layers[layer];

            println!("Layer {}", layer);
            let layer_matrix_product = &matrix_products[layer];
            let layer_activation = &activations[layer];
            println!("Layer activation {}", layer_activation);
            println!("layer weights {}", layer_weights);

            for row in 0..layer_weights.rows() {
                let f_derivative = sigmoid_derivative(layer_matrix_product.get(row, 0));
                println!("f_derivative {}", f_derivative);
                let target_diff = if layer == self.layers.len() - 1 {
                    y[row] - layer_activation.get(row, 0)
                } else {
                    let next_weights = &self.layers[layer + 1];
                    let mut sum = 0.0;
                    println!("MU");
                    println!("next errors {:?}", layer_diffs[layer + 1]);
                    println!("next_weights {}", next_weights);
                    for k in 0..next_weights.rows() {
                        let next_weight = next_weights.get(k, row);
                        let next_diff: f32 = layer_diffs[layer + 1][k];
                        println!("next_weight {} next_diff {}", next_weight, next_diff);
                        sum += next_weight * next_diff;
                    }
                    println!("END-MU");
                    sum
                };

                println!("layer {} row {} target_diff {}", layer, row, target_diff);
                let delta_pi = f_derivative * target_diff;
                layer_diffs[layer].push(delta_pi);

                for col in 0..layer_weights.cols() {
                    println!("row {} col {}", row, col);
                    println!("layer act {}", layer_activation);
                    let a_pj = {
                        if layer == 0 {
                            x.get(col, 0)
                        } else {
                            activations[layer - 1].get(col, 0)
                        }
                    };
                    let delta_w_ij = learning_rate * delta_pi * a_pj;
                    println!(
                        "layer {} row {} col {} delta_w_ij {}",
                        layer, row, col, delta_w_ij
                    );
                    weight_deltas[layer].set(row, col, delta_w_ij);
                }
            }
        }

        for (layer, diffs) in layer_diffs.iter().enumerate() {
            println!("DEBUG Layer {} diffs {:?}", layer, diffs);
        }
        for layer in 0..self.layers.len() {
            match &self.layers[layer] + &weight_deltas[layer] {
                Ok(matrix) => {
                    self.layers[layer] = matrix;
                }
                _ => (),
            }
        }
    }

    fn compute_error(&self, y: &Vec<f32>, output: &Vec<f32>) -> f32 {
        let mut error = 0.0;
        for i in 0..y.len() {
            let diff = y[i] - output[i];
            error += diff.powf(2.0);
        }
        error * 0.5
    }

    pub fn predict_many(&self, inputs: &Vec<Vec<f32>>) -> Vec<Vec<f32>> {
        inputs.iter().map(|x| self.predict(x)).collect()
    }

    pub fn predict(&self, x: &Vec<f32>) -> Vec<f32> {
        let x = x.clone();
        // TODO add constant bias
        // Add a constant for bias
        //x.push(1.0);
        let x = Matrix::new(x.len(), 1, x);
        let mut previous_activation = x;

        for layer_weights in self.layers.iter() {
            let matrix_product = layer_weights * &previous_activation;

            match matrix_product {
                Ok(matrix_product) => {
                    let mut activation = matrix_product.clone();
                    for row in 0..activation.rows() {
                        for col in 0..activation.cols() {
                            activation.set(row, col, sigmoid(matrix_product.get(row, col)));
                        }
                    }
                    previous_activation = activation;
                }
                _ => {
                    println!("Incompatible shapes in matrix multiplication");
                    println!("Between  W {} and A {}", layer_weights, previous_activation,);
                }
            }
        }

        let output: Vec<f32> = previous_activation.into();
        output
    }
}
