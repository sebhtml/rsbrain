use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    sync::Arc,
    thread::JoinHandle,
};

use crate::{execution_unit::ExecutionUnit, tensor::Error, Instruction};
#[cfg(test)]
mod tests;

pub struct Stream {
    pub id: usize,
    pub state: StreamState,
    pub dependencies: Vec<usize>,
    pub instructions: Arc<Vec<usize>>,
}

/// Maker sure that no instruction writes to machine inputs.
pub fn verify_machine_inputs(machine_inputs: &[usize], instructions: &[(Vec<usize>, Vec<usize>)]) {
    for machine_input in machine_inputs {
        for (i, (_, outputs)) in instructions.iter().enumerate() {
            if outputs.contains(machine_input) {
                panic!(
                    "[assign_streams] PROBLEM-0001 instruction {} writes ot machine input {} !",
                    i, machine_input
                );
            }
        }
    }
}

/// Group <N> instructions in <M> streams using a dependency analysis.
pub fn make_streams(instructions: &[(Vec<usize>, Vec<usize>)]) -> Vec<Stream> {
    // A list of dependencies for each instruction.
    let instruction_dependencies = get_instruction_dependencies(instructions);

    //#[cfg(feature = "verbose_streams")]
    for (i, i_dependencies) in instruction_dependencies.iter().enumerate() {
        println!(
            "[assign_streams] INSTRUCTION_DEPENDENCIES  instruction: {},  write_before_read: {:?},  read_before_write: {:?},  write_before_write: {:?}",
            i,
            i_dependencies.write_before_read,
            i_dependencies.read_before_write,
            i_dependencies.write_before_write,
        );

        let write_before_read = i_dependencies.write_before_read.len();
        let minimum_write_before_read = 4;
        if write_before_read >= minimum_write_before_read {
            println!(
                "instruction {} is perhaps a GOLD NUGGET of parallelism with write_before_read = {}",
                i,
                write_before_read
            );
        }
    }

    let instruction_streams = assign_instructions_to_streams(&instruction_dependencies);

    #[cfg(feature = "verbose_streams")]
    {
        println!("Instruction streams");
        for (i, stream) in instruction_streams.iter().enumerate() {
            println!("Instruction {}  stream {}", i, stream);
        }
    }

    let max_stream = instruction_streams.iter().max();
    let stream_count = match max_stream {
        Some(max_stream) => max_stream + 1,
        None => 0,
    };
    let mut stream_instructions = vec![vec![]; stream_count];
    for (i_inst, i_stream) in instruction_streams.iter().enumerate() {
        stream_instructions[*i_stream].push(i_inst);
    }

    let mut streams: Vec<Stream> = vec![];
    for i in 0..stream_instructions.len() {
        let instructions = stream_instructions[i].clone();
        let stream = Stream {
            id: i,
            state: Default::default(),
            dependencies: Default::default(),
            instructions: instructions.into(),
        };
        streams.push(stream);
    }

    // Assign stream dependencies
    for i in 0..streams.len() {
        let stream_instructions = &streams[i].instructions;
        let first_instruction = stream_instructions[0];
        let dependency_instructions = &instruction_dependencies[first_instruction];
        let mut dependency_instructions = vec![
            dependency_instructions.read_before_write.clone(),
            dependency_instructions.write_before_read.clone(),
            dependency_instructions.write_before_write.clone(),
        ]
        .concat();
        dependency_instructions.sort();
        dependency_instructions.dedup();
        let dependency_streams = dependency_instructions
            .iter()
            .map(|i| instruction_streams[*i])
            .collect::<Vec<_>>();
        streams[i].dependencies = dependency_streams;
    }

    #[cfg(feature = "verbose_streams")]
    for stream in streams.iter() {
        println!("STREAM {}", stream);
    }

    streams
}

fn get_instruction_dependencies(instructions: &[(Vec<usize>, Vec<usize>)]) -> Vec<Dependencies> {
    let mut dependencies = vec![Dependencies::default(); instructions.len()];
    for (i, (i_inputs, i_outputs)) in instructions.iter().enumerate() {
        //----------------------------
        // Write(operandX) and Read(operandX) must not be re-ordered.
        //----------------------------

        // Read of input depends on previous write.
        for i_input in i_inputs.iter() {
            // find the closest prior instruction that writes to his operand.
            // Then add it to the dependencies.
            let j_range = 0..i;
            for j in j_range.rev() {
                let j_outputs = &instructions[j].1;

                if j_outputs.contains(&i_input) {
                    dependencies[i].write_before_read.push(j);
                    break;
                }
            }
        }

        //----------------------------
        // Write(operandX) and Write(operandX) must not be re-ordered.
        //----------------------------

        // Write of output depends on previous write.
        // If previous instruction j writes to the same output as instruction i,
        // the order of the writes must be preserved.
        for i_output in i_outputs.iter() {
            // find the closest prior instruction that writes to his operand.
            // Then add it to the dependencies.
            let j_range = 0..i;
            for j in j_range.rev() {
                let j_outputs = &instructions[j].1;

                if j_outputs.contains(&i_output) {
                    dependencies[i].write_before_write.push(j);
                    break;
                }
            }
        }

        //----------------------------
        // Read(operandX) and Write(operandX) must not be re-ordered.
        //----------------------------
        // Write of output depends on previous read.
        // If we write to the operand before the previous read,
        // then the previous read will read a bad value from the future.
        // If previous instruction j reads to the same output as instruction i,
        // the order of the read-then-write must be preserved.
        for i_output in i_outputs.iter() {
            // find the closest prior instruction that writes to his operand.
            // Then add it to the dependencies.
            let j_range = 0..i;
            for j in j_range.rev() {
                let j_inputs = &instructions[j].0;

                if j_inputs.contains(&i_output) {
                    dependencies[i].read_before_write.push(j);
                    break;
                }
            }
        }
    }
    dependencies
}

fn assign_instructions_to_streams(instruction_dependencies: &[Dependencies]) -> Vec<usize> {
    let no_stream = usize::MAX;
    let n = instruction_dependencies.len();
    let mut instruction_streams: Vec<usize> = vec![no_stream; n];
    let mut next_stream = 0;
    for (i_inst, i_deps) in instruction_dependencies.iter().enumerate() {
        let mut i_deps = vec![
            i_deps.write_before_read.clone(),
            i_deps.read_before_write.clone(),
            i_deps.write_before_write.clone(),
        ]
        .concat();
        i_deps.sort();
        i_deps.dedup();
        if i_deps.len() == 1 {
            let dependency_instruction = i_deps[0];
            let stream = instruction_streams[dependency_instruction];
            if stream == no_stream {
                panic!("Prior instruction has no assigned stream");
            }
            instruction_streams[i_inst] = stream;
        } else {
            // Instructions with 2 or more dependencies can wait for their dependencies,
            // and then run.
            instruction_streams[i_inst] = next_stream;
            next_stream += 1;
        }
    }

    #[cfg(feature = "verbose_streams")]
    for (i_inst, i_stream) in instruction_streams.iter().enumerate() {
        println!(
            "[assign_streams] INSTRUCTION_STREAM  instruction: {},  stream: {}",
            i_inst, i_stream,
        );
    }

    instruction_streams
}

impl Display for Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _ = write!(f, "Stream id: {}", self.id,);
        let _ = write!(f, "\n");
        let _ = write!(f, "state: {}", self.state,);
        let _ = write!(f, "\n");
        let _ = write!(f, "dependencies_len: {}", self.dependencies.len(),);
        let _ = write!(f, "\n");
        let _ = write!(f, "dependencies: {:?}", self.dependencies,);
        let _ = write!(f, "\n");
        let _ = write!(f, "instructions_len: {:?}", self.instructions.len(),);
        let _ = write!(f, "\n");
        let _ = write!(f, "instructions: {:?}", self.instructions,);
        let result = write!(f, "\n");
        result
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum StreamState {
    Unreached,
    Spawned,
    Joined,
}

impl Into<String> for &StreamState {
    fn into(self) -> String {
        match self {
            StreamState::Unreached => "Unreached",
            StreamState::Spawned => "Spawned",
            StreamState::Joined => "Joined",
        }
        .into()
    }
}

impl Display for StreamState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let as_string: String = self.into();
        let result = write!(f, "{}", as_string);
        result
    }
}

impl Default for StreamState {
    fn default() -> Self {
        StreamState::Unreached
    }
}

fn join_stream(
    stream: usize,
    streams: &mut Vec<Stream>,
    _threads: &mut Vec<Option<JoinHandle<Result<(), Error>>>>,
    active_streams: &mut BTreeSet<usize>,
) -> Result<(), Error> {
    debug_assert_eq!(StreamState::Spawned, streams[stream].state);
    /*
    let thread = threads[stream].take();
    if let Some(thread) = thread {
        match thread.join() {
            Ok(result) => match result {
                Ok(_) => (),
                Err(err) => return Err(err),
            },
            Err(_) => return Err(error!(ErrorEnum::UnsupportedOperation)),
        }
    }
     */
    let new_state = StreamState::Joined;
    #[cfg(feature = "verbose_streams")]
    println!(
        "Transition stream {}  {} -> {}",
        stream, streams[stream].state, new_state
    );
    streams[stream].state = new_state;
    active_streams.remove(&stream);
    #[cfg(feature = "verbose_streams")]
    println!("active_streams {}", active_streams.len());
    Ok(())
}

fn spawn_stream(
    stream: usize,
    streams: &mut Vec<Stream>,
    threads: &mut Vec<Option<JoinHandle<Result<(), Error>>>>,
    instructions: &Arc<Vec<Instruction>>,
    active_streams: &mut BTreeSet<usize>,
) -> Result<(), Error> {
    debug_assert_eq!(StreamState::Unreached, streams[stream].state);
    let new_state = StreamState::Spawned;
    #[cfg(feature = "verbose_streams")]
    println!(
        "Transition stream {}  {} -> {}",
        stream, streams[stream].state, new_state
    );
    streams[stream].state = new_state;
    active_streams.insert(stream);
    #[cfg(feature = "verbose_streams")]
    println!("active_streams {}", active_streams.len());

    let stream_instructions = streams[stream].instructions.clone();
    let instructions = instructions.clone();

    ExecutionUnit::execute(stream_instructions, instructions)?;
    /*
    let spawned_thread =
        thread::spawn(|| ExecutionUnit::execute(stream_instructions, instructions));
    threads[stream] = Some(spawned_thread);
    */
    Ok(())
}

pub fn execute_streams(
    streams: &mut Vec<Stream>,
    instructions: &Arc<Vec<Instruction>>,
    max_concurrent_streams: usize,
) -> Result<(), Error> {
    let mut threads: Vec<Option<JoinHandle<Result<(), Error>>>> = vec![];
    for _ in 0..streams.len() {
        threads.push(None);
    }
    let range = 0..streams.len();
    let mut active_streams = BTreeSet::new();
    for i in range.clone().into_iter() {
        let is_unreached = streams[i].state == StreamState::Unreached;
        if is_unreached {
            // Join each dependency
            let n = streams[i].dependencies.len();
            for j in 0..n {
                let dependency = streams[i].dependencies[j];
                if streams[dependency].state == StreamState::Spawned {
                    join_stream(dependency, streams, &mut threads, &mut active_streams)?;
                } else if streams[dependency].state == StreamState::Joined {
                    #[cfg(feature = "verbose_streams")]
                    println!(
                        "note stream {} is already {}",
                        dependency,
                        StreamState::Joined
                    );
                } else {
                    panic!("Can not join unspawned stream {}", dependency);
                }
            }

            if active_streams.len() == max_concurrent_streams {
                // Join the oldest active stream before spawning this one.
                let oldest = active_streams.iter().min().map(|x| *x);
                if let Some(oldest) = oldest {
                    join_stream(oldest, streams, &mut threads, &mut active_streams)?;
                }
            }
            spawn_stream(i, streams, &mut threads, instructions, &mut active_streams)?;
        } else {
            panic!("Can not spawn stream {} because it is not unreached", i);
        }
    }
    for i in range {
        if streams[i].state == StreamState::Spawned {
            join_stream(i, streams, &mut threads, &mut active_streams)?;
        }
    }
    debug_assert_eq!(0, active_streams.len());

    Ok(())
}

pub fn reset_streams(streams: &mut Vec<Stream>) {
    for i in 0..streams.len() {
        streams[i].state = StreamState::Unreached;
    }
}

pub fn make_simple_instructions(instructions: &Vec<Instruction>) -> Vec<(Vec<usize>, Vec<usize>)> {
    let instructions = instructions
        .iter()
        .map(|instruction| {
            let inputs = instruction
                .inputs()
                .iter()
                .map(|x| x.name())
                .collect::<Vec<_>>();
            let outputs = instruction
                .outputs()
                .iter()
                .map(|x| x.name())
                .collect::<Vec<_>>();
            (inputs, outputs)
        })
        .collect::<Vec<_>>();
    instructions
}

#[derive(Clone, Debug)]
pub struct Dependencies {
    pub write_before_read: Vec<usize>,
    pub write_before_write: Vec<usize>,
    pub read_before_write: Vec<usize>,
}

impl Default for Dependencies {
    fn default() -> Self {
        Self {
            write_before_read: Default::default(),
            write_before_write: Default::default(),
            read_before_write: Default::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum Access {
    Read,
    Write,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct Transaction {
    pub instruction: usize,
    pub operand: usize,
    pub access: Access,
}

pub fn get_instruction_transactions(
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

pub fn get_all_instruction_transactions(
    instructions: &[(Vec<usize>, Vec<usize>)],
) -> Vec<Transaction> {
    let mut transactions = vec![];
    for (instruction, (inputs, outputs)) in instructions.iter().enumerate() {
        let mut operand_transactions = get_instruction_transactions(instruction, inputs, outputs);
        transactions.extend_from_slice(&mut operand_transactions);
    }
    transactions
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
pub fn get_operand_transaction_pairs(
    access: &Access,
    prior_access: &Access,
    transactions: &[Transaction],
) -> BTreeMap<usize, Vec<(Transaction, Transaction)>> {
    // Group transactions per operand.
    let operand_transactions = group_by_operand(transactions);
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

fn group_by_operand(transactions: &[Transaction]) -> BTreeMap<usize, Vec<Transaction>> {
    let mut operand_transactions = BTreeMap::<usize, Vec<Transaction>>::new();
    for transaction in transactions.iter() {
        let operand = transaction.operand;
        operand_transactions
            .entry(operand)
            .or_default()
            .push(transaction.to_owned());
    }
    operand_transactions
}
