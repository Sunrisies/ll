use clap::{Parser, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::Path;
use std::result::Result;

#[derive(Parser, Debug)]
#[command(version, author, about, long_about = None)]
struct Cli {
    /// 指定要列出的文件或目录
    #[arg(default_value = ".", value_name = "FILE")]
    file: String,

    /// 启用详细模式
    #[arg(short = 'l', long = "long", help = "使用长列表格式")]
    long_format: bool,

    /// 启用人类可读的文件大小
    #[arg(short = 'h', long = "human-readable", help = "打印 如1K、234M、2G等。")]
    human_readable: bool,

    /// 显示隐藏文件
    #[arg(short = 'a', long = "all", help = "不要忽略以开头的条目 .")]
    all: bool,

    /// 显示程序运行时间
    #[arg(short = 't', long = "time")]
    show_time: bool,
}

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

fn list_directory(path: &Path, args: &Cli) {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("ls: cannot access '{}': {}", path.display(), e);
            return;
        }
    };

    let mut files: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        files.push(file_name);
    }

    files.sort();
    let scan_pb = progress_bar_init(None).unwrap();

    let mut entries = Vec::new(); // 新增存储条目信息的结构

    if args.long_format {
        let process_pb = progress_bar_init(None).unwrap(); // 修改为不传入具体数值
        process_pb.set_message("处理中..."); // 设置固定提示信息

        for (i, file) in files.iter().enumerate() {
            // process_pb.set_message(format!("处理 {}", file));

            // 在文件处理循环结束后保留进度条不清理
            // process_pb.finish_and_clear(); // 注释此行
            process_pb.tick();
            let file_path = path.join(&file);
            let metadata = match file_path.metadata() {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("ls: cannot access '{}': {}", file_path.display(), e);
                    continue;
                }
            };

            // 收集条目信息
            entries.push((
                if metadata.is_dir() { "d" } else { "-" },
                format!(
                    "{}-{}-{}",
                    if metadata.permissions().readonly() {
                        "r"
                    } else {
                        " "
                    },
                    "w",
                    "x"
                ),
                if metadata.is_dir() {
                    let (raw, converted) =
                        calculate_dir_size(&file_path, args.human_readable, &process_pb);
                    converted // 转换后的可读格式
                } else if args.human_readable {
                    human_readable_size(metadata.len())
                } else {
                    metadata.len().to_string()
                },
                // 新增原始大小字段（目录用raw，文件用metadata.len()）
                if metadata.is_dir() {
                    calculate_dir_size(&file_path, false, &process_pb).0
                } else {
                    metadata.len()
                },
                file_path.display().to_string(),
            ));
        }

        process_pb.finish_and_clear();
        let sum = entries.clone();
        let mut sum_size = 0;
        for entry in &entries {
            sum_size += entry.3; // 使用第4个字段的原始大小
        }
        // 打印条目信息
        for (i, entry) in entries.iter().enumerate() {
            println!("{:>5} {:>10} {:>10} {}", entry.0, entry.1, entry.2, entry.4);
        }
        println!(
            "\n总数量: {} 个条目 | 总大小: {}",
            entries.len(),
            human_readable_size(sum_size)
        );
    } else {
        for file in files {
            println!("{}", file);
        }
    }
    scan_pb.finish_and_clear(); // 完成后清理进度条
}

fn human_readable_size(bytes: u64) -> String {
    // 定义单位数组
    let units = ["B", "KB", "MB", "GB", "TB"];
    // 将字节数转换为浮点数
    let mut size = bytes as f64;
    // 初始化单位索引
    let mut unit = 0;

    while size >= 1024.0 && unit < units.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    // 添加目录大小处理
    if bytes == 0 {
        return String::from("0B");
    }

    format!("{:.1}{}", size, units[unit])
}

fn calculate_dir_size(path: &Path, human_readable: bool, main_pb: &ProgressBar) -> (u64, String) {
    fn inner_calculate(p: &Path, pb: &ProgressBar) -> u64 {
        fs::read_dir(p)
            .map(|entries| {
                entries
                    .filter_map(|e| {
                        pb.tick();
                        e.ok()
                    })
                    .map(|e| {
                        let md = match e.metadata() {
                            Ok(metadata) => metadata,
                            Err(_) => return 0,
                        };
                        if md.is_dir() {
                            inner_calculate(&e.path(), pb)
                        } else {
                            md.len()
                        }
                    })
                    .sum()
            })
            .unwrap_or(0)
    }

    main_pb.set_message(format!("计算 {}...", path.display()));
    let total = inner_calculate(path, main_pb);
    main_pb.set_message("处理中...");

    let converted = if human_readable {
        human_readable_size(total)
    } else {
        total.to_string()
    };
    (total, converted)
}

// 引入 ProgressBar 类型，假设它来自 indicatif 库
fn progress_bar_init(total_files: Option<u64>) -> Result<ProgressBar, Box<dyn std::error::Error>> {
    let pb = match total_files {
        Some(total) => ProgressBar::new(total),
        None => ProgressBar::new_spinner(),
    };

    // 修改进度条样式模板
    let style = match total_files {
        Some(_) => ProgressStyle::default_bar().template("{spinner:.green} {msg}")?,
        None => ProgressStyle::default_spinner().template("{spinner:.green} {msg}")?,
    };

    pb.set_style(style.progress_chars("#>-"));
    Ok(pb)
}
