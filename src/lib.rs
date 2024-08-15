mod bench;
mod benches;
mod space;
mod stat;
mod rnd;

#[repr(C)]
pub struct Config {
    retries: usize,
    verbose: bool,
    // Update bench config.
    // TODO: possibility to choose benchmark.

    // 0 - copy
    // 1 - splices
    method: usize,
    block_size: usize,
    block_num: usize,
    // [0, 1]
    update_percentage: f32,
    transaction_per_block: bool,
}

#[no_mangle]
pub extern "C" fn run(config: Config) {
    let state = benches::update_column::State::try_new(benches::update_column::Config {
        block_size: config.block_size,
        block_num: config.block_num,
        update_percentage: config.update_percentage,
        transaction_per_block: config.transaction_per_block,
    })
    .expect("state is ok");

    let durations = bench::run_bench(
        config.retries,
        if config.method == 0 {
            benches::update_column::run_copy
        } else {
            benches::update_column::run_splices
        },
        benches::update_column::setup,
        benches::update_column::teardown,
        state,
    );

    let stat::Stats { mean, variance } = stat::Stats::try_from_slice(
        durations
            .iter()
            .map(|it| it.as_micros() as f64)
            .collect::<Vec<_>>()
            .as_ref(),
    )
    .expect("retries > 0");

    let updated_entries = ((config.block_size as f32) * config.update_percentage).floor() as usize;

    if config.verbose {
        println!("*---------------------------------------------*");
        println!("| update benchmark");
        println!(
            "| method:            {}",
            if config.method == 0 {
                "copy"
            } else {
                "splices"
            }
        );
        println!("| block_num:         {}", config.block_num);
        println!("| block_size:        {}", config.block_size);
        println!("| updated_entries:   {}", updated_entries);
        println!("| retries:           {}", config.retries);
        println!("*---------------------------------------------*");
        println!("| mean:              {} [us]", mean);
        println!("| std:            +- {} [us]", variance.sqrt());
        println!("*---------------------------------------------*\n");
    } else {
        println!("{} {} {}", updated_entries, mean, variance.sqrt())
    }
}
