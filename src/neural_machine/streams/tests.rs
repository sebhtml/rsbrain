use std::collections::{BTreeMap, BTreeSet};

use crate::{
    mega_man_attention::MegaManAttentionModel, tensor::Error, Adam, BinaryOperator, Category,
    Device, NeuralMachine, OptimizerTrait, SoftmaxCrossEntropyLoss, Tokenizer, TokenizerTrait,
    UnaryModel,
};

use super::{make_simple_instructions, make_streams, Stream};

fn get_test_instructions() -> Result<Vec<(Vec<usize>, Vec<usize>)>, Error> {
    let device = Device::default();
    let tokenizer = Tokenizer::ascii_tokenizer();
    let vocab_size = tokenizer.vocab_size();
    let sequence_length = 32;
    let model = MegaManAttentionModel::new(&device, sequence_length, vocab_size)?;
    let model: Box<dyn UnaryModel> = Box::new(model);
    let loss_operator = SoftmaxCrossEntropyLoss::new(&device);
    let loss_operator: Box<dyn BinaryOperator> = Box::new(loss_operator);
    let clipped_gradient_norm = 1.0;
    let learning_rate = 0.05;
    let optimizer = Adam::new(learning_rate, 0.9, 0.98, 1e-9);
    let optimizer: Box<dyn OptimizerTrait> = Box::new(optimizer);
    let neural_machine = NeuralMachine::<f32>::try_new(
        &device,
        &model,
        &loss_operator,
        clipped_gradient_norm,
        &optimizer,
    )?;
    let instructions = vec![
        neural_machine.instructions(&Category::Inference),
        neural_machine.instructions(&Category::Loss),
        neural_machine.instructions(&Category::Gradient),
        neural_machine.instructions(&Category::Optimization),
    ]
    .concat();
    let simple_instructions = make_simple_instructions(&instructions);
    Ok(simple_instructions)
}

#[test]
fn each_instruction_is_executed_exactly_once() {
    let instructions = get_test_instructions().unwrap();
    let expected_instructions = (0..instructions.len()).collect::<Vec<_>>();
    let streams = make_streams(&instructions);
    let mut actual_instructions = streams
        .iter()
        .map(|x| x.instructions.clone())
        .collect::<Vec<_>>()
        .concat();
    actual_instructions.sort();
    assert_eq!(expected_instructions, actual_instructions);
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
enum Access {
    Read,
    Write,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
struct Transaction {
    pub instruction: usize,
    pub operand: usize,
    pub access: Access,
}

fn get_instruction_transactions(
    instruction: usize,
    inputs: &[usize],
    outputs: &[usize],
) -> Vec<Transaction> {
    let mut transactions = vec![];
    for operand in inputs {
        let transaction = Transaction {
            instruction,
            operand: *operand,
            access: Access::Read,
        };
        transactions.push(transaction);
    }
    for operand in outputs {
        let transaction = Transaction {
            instruction,
            operand: *operand,
            access: Access::Write,
        };
        transactions.push(transaction);
    }
    transactions
}

fn get_all_instruction_transactions(instructions: &[(Vec<usize>, Vec<usize>)]) -> Vec<Transaction> {
    let mut transactions = vec![];
    for (instruction, (inputs, outputs)) in instructions.iter().enumerate() {
        let mut operand_transactions = get_instruction_transactions(instruction, inputs, outputs);
        transactions.extend_from_slice(&mut operand_transactions);
    }
    transactions
}

/// Simulate an execution of streams and emit operand transactions.
fn spawn_and_join_streams(
    streams: &[Stream],
    instructions: &[(Vec<usize>, Vec<usize>)],
) -> Vec<Transaction> {
    let mut actual_transactions = vec![];
    let mut unreached_streams = BTreeSet::<usize>::new();
    for i in 0..streams.len() {
        unreached_streams.insert(i);
    }
    let mut spawned_streams = BTreeSet::<usize>::new();
    let mut joined_streams = BTreeSet::<usize>::new();
    while joined_streams.len() != streams.len() {
        let mut stream_to_spawn: Option<usize> = None;
        // Find a stream that can be spawned.
        for unreached_stream in unreached_streams.iter() {
            let mut can_spawn = true;
            let dependencies = &streams[*unreached_stream].dependencies;
            for dependency in dependencies {
                if !joined_streams.contains(dependency) {
                    can_spawn = false;
                    break;
                }
            }
            if can_spawn {
                stream_to_spawn = Some(*unreached_stream);
                break;
            }
        }
        if let Some(stream_to_spawn) = stream_to_spawn {
            // Spawn it.
            unreached_streams.remove(&stream_to_spawn);
            spawned_streams.insert(stream_to_spawn);
            // Emit transactions on the execution unit pipeline.
            let stream_instructions = &streams[stream_to_spawn].instructions;
            for instruction in stream_instructions.iter() {
                let instruction = *instruction;
                let (inputs, outputs) = &instructions[instruction];
                let mut instruction_transactions =
                    get_instruction_transactions(instruction, inputs, outputs);
                actual_transactions.extend_from_slice(&mut instruction_transactions);
            }
            // Immediately join the thread.
            joined_streams.insert(stream_to_spawn);
        }
    }
    actual_transactions
}

// Example: for each read, find the prior write.
// Basically there are read accesses and write accesses.
// Here are the 4 pillars of the memory model:
// - a read has a prior write and it must remain the same. Changing the prior write makes the result incorrect.
// - a write has a prior write and it must remain the same. Changing the prior write makes the result incorrect.
// - a write has a prior read and it must remain the same. Changing the prior read makes the result incorrect.
// - a read has a prior read and it can change. Changing the prior read is allowed.
//        Example, if instructions 1, 2, 3 read operand 44, all those orderings are valid ones:
//           - 1, 2, 3
//           - 3, 2, 1
//           - 2, 1, 3
//           - ...
//       If we have 12 attention heads, that means that we can have 12 concurrent streams.
fn get_operand_transaction_pairs(
    access: &Access,
    prior_access: &Access,
    transactions: &[Transaction],
) -> BTreeMap<usize, Vec<(Transaction, Transaction)>> {
    // Group transactions per operand.
    let mut operand_transactions = BTreeMap::<usize, Vec<Transaction>>::new();
    for transaction in transactions.iter() {
        let operand = transaction.operand;
        operand_transactions
            .entry(operand)
            .or_default()
            .push(transaction.to_owned());
    }
    // For each read of an operand, find the most recent write before it­.
    let mut operand_pairs = BTreeMap::<usize, Vec<(Transaction, Transaction)>>::new();
    for (operand, transactions) in operand_transactions.iter() {
        for i in 0..transactions.len() {
            let transaction_i = &transactions[i];
            if &transaction_i.access == access {
                // Find the most recent write to this operand that happened in the past.
                for j in (0..i).rev() {
                    let transaction_j = &transactions[j];
                    if &transaction_j.access == prior_access {
                        let pair = (transaction_i.to_owned(), transaction_j.to_owned());
                        operand_pairs.entry(*operand).or_default().push(pair);
                        break;
                    }
                }
            }
        }
    }
    for (_, pairs) in operand_pairs.iter_mut() {
        // The tests use == so sorting makes the tests pass.
        pairs.sort();
    }
    operand_pairs
}

#[test]
fn the_instructions_length_and_streams_length_are_correct() {
    let instructions = get_test_instructions().unwrap();
    assert_eq!(2810, instructions.len());
    let streams = make_streams(&instructions);
    assert_eq!(1749, streams.len());
}

#[test]
fn reads_and_writes_of_same_operand_are_not_reordered() {
    let access = Access::Read;
    let prior_access = Access::Write;
    let instructions = get_test_instructions().unwrap();
    let expected_transactions = get_all_instruction_transactions(&instructions);
    let expected_read_write_pairs =
        get_operand_transaction_pairs(&access, &prior_access, &expected_transactions);

    let actual_streams = make_streams(&instructions);
    let actual_transactions = spawn_and_join_streams(&actual_streams, &instructions);
    let actual_read_write_pairs =
        get_operand_transaction_pairs(&access, &prior_access, &actual_transactions);

    for (operand, expected_pairs) in expected_read_write_pairs.iter() {
        let actual_pairs = actual_read_write_pairs.get(operand).unwrap();
        assert_eq!(expected_pairs, actual_pairs);
    }
}

#[test]
fn writes_and_writes_of_same_operand_are_not_reordered() {
    let access = Access::Write;
    let prior_access = Access::Write;
    let instructions = get_test_instructions().unwrap();
    let expected_transactions = get_all_instruction_transactions(&instructions);
    let expected_read_write_pairs =
        get_operand_transaction_pairs(&access, &prior_access, &expected_transactions);

    let actual_streams = make_streams(&instructions);
    let actual_transactions = spawn_and_join_streams(&actual_streams, &instructions);
    let actual_read_write_pairs =
        get_operand_transaction_pairs(&access, &prior_access, &actual_transactions);

    for (operand, expected_pairs) in expected_read_write_pairs.iter() {
        let actual_pairs = actual_read_write_pairs.get(operand).unwrap();
        assert_eq!(expected_pairs, actual_pairs);
    }
}

#[test]
fn writes_and_reads_of_same_operand_are_not_reordered() {
    let access = Access::Write;
    let prior_access = Access::Read;
    let instructions = get_test_instructions().unwrap();
    let expected_transactions = get_all_instruction_transactions(&instructions);
    let expected_read_write_pairs =
        get_operand_transaction_pairs(&access, &prior_access, &expected_transactions);

    let actual_streams = make_streams(&instructions);
    let actual_transactions = spawn_and_join_streams(&actual_streams, &instructions);
    let actual_read_write_pairs =
        get_operand_transaction_pairs(&access, &prior_access, &actual_transactions);

    for (operand, expected_pairs) in expected_read_write_pairs.iter() {
        let actual_pairs = actual_read_write_pairs.get(operand).unwrap();
        assert_eq!(expected_pairs, actual_pairs);
    }
}
