use crate::{CrossEntropyLoss, Device, Tokenizer};
use crate::{DatasetDetails, Error};
mod model;
use model::*;

use super::load_examples;

pub fn load_dataset(device: &Device) -> Result<DatasetDetails, Error> {
    let file_path = "data/Mega_Man.txt";
    let model = Model::new(device);
    let vocab_size = model.vocab_size();
    let mut tokenizer = if vocab_size == 256 {
        Tokenizer::ascii_tokenizer()
    } else {
        Tokenizer::byte_pair_encoding()
    };

    let input_sequence_length = model.sequence_length();
    let output_sequence_length = input_sequence_length;
    let examples = load_examples(
        &device,
        file_path,
        input_sequence_length,
        output_sequence_length,
        vocab_size,
        &mut tokenizer,
    )?;

    let details = DatasetDetails {
        device: device.clone(),
        tokenizer,
        examples,
        model: Box::new(model),
        epochs: 300,
        progress: 100,
        loss_function_name: Box::new(CrossEntropyLoss::new(device)),
        initial_total_error_min: 50.0,
        final_total_error_max: 0.002,
        learning_rate: 0.5,
    };
    Ok(details)
}