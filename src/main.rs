use clap::Parser;
use std::path::Path;

mod dir_listing;
mod models;
mod utils;

use dir_listing::list_directory;
use models::Cli;

fn main() {
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
        println!("\n运行时间: {:.3}秒", duration.as_secs_f64());
    }
}
