use clap::Parser;
use ll::log::init_logger;
use std::path::Path;

mod dir_listing;
mod models;
mod utils;
use dir_listing::list_directory;
use models::Cli;

fn main() -> Result<(), anyhow::Error> {
    init_logger();
    let start_time = std::time::Instant::now();
    let args = Cli::parse();
    let path = Path::new(&args.file);
    if path.is_dir() {
        list_directory(path, &args);
    } else {
        println!("{}", path.display());
    }

    if args.show_time {
        let duration = start_time.elapsed();
        log::warn!(
            "\n运行时间: {:.6}秒 ({}毫秒, {}纳秒)",
            duration.as_secs_f64(),
            duration.as_millis(),
            duration.as_nanos()
        );
    }
    Ok(())
}
