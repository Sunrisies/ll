use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use ll::dir_listing::list_directory;
use ll::models::Cli;
use std::hint::black_box;
use std::path::Path;

fn bench_directory_listing(c: &mut Criterion) {
    let test_dirs = vec![
        ("small_dir", "E:/"),
        // ("medium_dir", "d:/project/project"),
        // ("large_dir", "d:/project"),
        // ("system_dir", "c:/"),
    ];

    let mut group = c.benchmark_group("directory_listing");

    for (name, dir_path) in test_dirs {
        let path = Path::new(dir_path);
        let args: Cli = Cli {
            file: dir_path.to_string(),
            long_format: true,
            human_readable: true,
            all: true,
            show_time: true,
            parallel: true,
            sort: true,
            name: None, // 移除name参数，测试正常列表功能
            full_path: false,
        };

        println!("开始测试 {} 目录: {}", name, dir_path);
        group.bench_with_input(BenchmarkId::new("optimized", name), &path, |b, path| {
            b.iter(|| list_directory(black_box(path), black_box(&args)))
        });
    }

    group.finish();
}

// 测试并行与非并行处理的性能差异
fn bench_parallel_vs_serial(c: &mut Criterion) {
    let path = Path::new("d:/project");

    // 并行处理
    let parallel_args: Cli = Cli {
        file: "d:/project".to_string(),
        long_format: true,
        human_readable: true,
        all: true,
        show_time: true,
        parallel: true,
        sort: true,
        name: None,
        full_path: false,
    };

    // 串行处理
    let serial_args: Cli = Cli {
        file: "d:/project".to_string(),
        long_format: true,
        human_readable: true,
        all: true,
        show_time: true,
        parallel: false,
        sort: true,
        name: None,
        full_path: false,
    };

    let mut group = c.benchmark_group("parallel_vs_serial");

    println!("开始测试并行处理");
    group.bench_function("parallel", |b| {
        b.iter(|| list_directory(black_box(path), black_box(&parallel_args)))
    });

    println!("开始测试串行处理");
    group.bench_function("serial", |b| {
        b.iter(|| list_directory(black_box(path), black_box(&serial_args)))
    });

    group.finish();
}

// 测试不同参数组合的性能
fn bench_parameter_combinations(c: &mut Criterion) {
    let path = Path::new("d:/project");

    let mut group = c.benchmark_group("parameter_combinations");

    // 测试不同参数组合
    for (name, parallel, human_readable, long_format) in vec![
        ("default", true, true, true),
        ("no_parallel", false, true, true),
        ("no_human_readable", true, false, true),
        ("no_long_format", true, true, false),
        ("minimal", false, false, false),
    ] {
        let args: Cli = Cli {
            file: "d:/project".to_string(),
            long_format,
            human_readable,
            all: true,
            show_time: true,
            parallel,
            sort: true,
            name: None,
            full_path: false,
        };

        println!("开始测试参数组合: {}", name);
        group.bench_with_input(BenchmarkId::new("params", name), &path, |b, path| {
            b.iter(|| list_directory(black_box(path), black_box(&args)))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_directory_listing // bench_parallel_vs_serial,
                            // bench_parameter_combinations
);
criterion_main!(benches);
