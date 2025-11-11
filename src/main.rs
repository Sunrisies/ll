use clap::Parser;
use std::path::Path;

mod dir_listing;
mod dir_listing_v2;
mod models;
mod my_benchmark;
mod utils;
use dir_listing::list_directory;
use dir_listing_v2::list_directory_v2;
use models::Cli;

fn main() {
    let start_time = std::time::Instant::now();
    let args = Cli::parse();
    let path = Path::new(&args.file);
    if path.is_dir() {
        // list_directory(path, &args);
        list_directory_v2(path, &args);
    } else {
        println!("{}", path.display());
    }

    if args.show_time {
        let duration = start_time.elapsed();
        println!("\n运行时间: {:.3}秒", duration.as_secs_f64());
    }
}
