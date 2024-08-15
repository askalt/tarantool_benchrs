use std::convert::Infallible;

use rand::Rng;
use serde_bytes::Bytes;

use crate::space::{BinSpace, Entry};

pub struct Config {
    pub block_size: usize,
    pub block_num: usize,
    // [0, 1]
    pub update_percentage: f32,
    pub transaction_per_block: bool,
}

struct Data {
    // size == column_size
    updates: Vec<u8>,
    // size == column_size
    update_mask: Vec<bool>,
}

impl Data {
    fn from_config(config: &Config) -> Self {
        let mut rng = rand::thread_rng();

        let to_update = ((config.block_size as f32) * config.update_percentage).floor() as usize;

        let mut update_mask = vec![false; config.block_size];
        let mut updates = vec![0; config.block_size];

        let update_indices =
            crate::rnd::generate_diff_sequence(&mut rng, 0, config.block_size - 1, to_update);
        for index in update_indices {
            let add_val = rng.gen_range(0..u8::MAX) as u8;
            update_mask[index] = true;
            updates[index] = add_val;
        }

        Data {
            updates: updates,
            update_mask: update_mask,
        }
    }
}

pub struct State {
    filled: bool,
    config: Config,
    space: BinSpace,
    data: Data,
    // Remember each step context to can validate result.
    id: usize,
    expected: Vec<u8>,
}

impl State {
    pub fn try_new(config: Config) -> Option<Self> {
        let space = BinSpace::try_new()?;
        let data = Data::from_config(&config);
        Some(Self {
            filled: false,
            config: config,
            space: space,
            data: data,
            id: 0,
            expected: vec![],
        })
    }
}

fn generate_initial_data(state: &mut State) {
    let mut rng = rand::thread_rng();
    let column_num = state.config.block_num;
    let column_size = state.config.block_size;

    // Insert random data to space.
    for i in 0..column_num {
        let mut new_column = vec![0; column_size];
        for j in 0..column_size {
            new_column[j] = rng.gen_range(0..u8::MAX) as u8;
            // new_column[j] = (j % (u8::MAX as usize)) as u8;
        }
        state.space.put(Entry::new(i, Bytes::new(&new_column)));
    }
}

pub fn setup(state: &mut State) {
    if !state.filled {
        generate_initial_data(state);
        state.filled = true;
    }

    let mut rng = rand::thread_rng();

    let id = rng.gen_range(0..state.config.block_num);
    let current = state.space.get(id).unwrap();

    // Apply changes to expected
    state.id = id;
    state.expected = Vec::from(current.data.as_ref());
    for (i, to_update) in state.data.update_mask.iter().enumerate() {
        if *to_update {
            let (res, _) = state.expected[i].overflowing_add(state.data.updates[i]);
            state.expected[i] = res;
        }
    }
}

/// Update column by copy data and build array from the beginning.
pub fn run_copy(state: &mut State) {
    // O(log ColumnNum) tree index seek + O(1) pointer conversion.
    let was = state.space.get(state.id).unwrap();
    let mut updated = Vec::with_capacity(state.config.block_size);

    // O (ColumnSize * sizeof(value)) for primitive.
    // O (sum |size_i|) for strings/arrays.
    // ~ BLOB_SIZE pushes to vec.
    for (i, to_update) in state.data.update_mask.iter().enumerate() {
        if !*to_update {
            updated.push(was.data[i]);
        } else {
            let (res, _) = was.data[i].overflowing_add(state.data.updates[i]);
            updated.push(res);
        }
    }
    state.space.put(Entry::new(state.id, Bytes::new(&updated)));
}

pub fn run_splices(state: &mut State) {
    // O(log ColumnNum) tree index seek + O(1) pointer conversion.
    let was = state.space.get(state.id).unwrap();

    let mut splices = vec![];

    // Make splices array.
    for (i, to_update) in state.data.update_mask.iter().enumerate() {
        if !*to_update {
            continue;
        }
        let (res, _) = was.data[i].overflowing_add(state.data.updates[i]);
        let offset = i * std::mem::size_of::<u8>();
        splices.push((offset, res));
    }

    // TODO: complexity.
    let apply_splices = || -> Result<(), Infallible> {
        for (offset, val) in splices {
            state
                .space
                .splice_bindata(state.id, offset, Bytes::new(&[val]))
        }
        Ok(())
    };

    // Note: can't run this update batch in the single transaction.
    // Memory limit is reached during transaction.
    // Why:
    if state.config.transaction_per_block {
        tarantool::transaction::transaction(apply_splices).unwrap();
    } else {
        apply_splices().unwrap();
    }
}

pub fn teardown(state: &mut State) {
    let current = state.space.get(state.id).unwrap();
    // Validate that data was updated.
    assert_eq!(&state.expected, current.data.as_ref());
}
