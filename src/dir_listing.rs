use super::models::{Cli, FileEntry};
use super::utils::{human_readable_size, progress_bar_init};
use comfy_table::{Cell, ContentArrangement, Table};
use indicatif::ProgressBar;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use std::sync::Arc;

pub fn calculate_dir_size(
    path: &Path,
    human_readable: bool,
    main_pb: &ProgressBar,
    parallel: bool,
) -> (u64, String) {
    fn inner_calculate(p: &Path, pb: &ProgressBar, parallel: bool) -> u64 {
        match fs::read_dir(p) {
            Ok(entries) => {
                let mut total_size = 0;
                let entries: Vec<_> = entries
                    .into_iter()
                    .filter_map(|e| {
                        pb.tick();
                        match e {
                            Ok(entry) => Some(entry),
                            Err(e) => {
                                eprintln!("无法读取目录项 {}: {}", p.display(), e);
                                None
                            }
                        }
                    })
                    .collect();

                if parallel {
                    // 使用并行处理
                    total_size += entries
                        .par_iter()
                        .map(|e| process_entry(e, pb, parallel))
                        .sum::<u64>();
                } else {
                    // 使用串行处理
                    total_size += entries
                        .iter()
                        .map(|e| process_entry(e, pb, parallel))
                        .sum::<u64>();
                }

                total_size
            }
            Err(e) => {
                eprintln!("无法读取目录 {}: {}", p.display(), e);
                0 // 返回0表示这个目录本身无法访问，但不影响父目录计算其他项
            }
        }
    }

    // 修改process_entry函数以处理DirEntry引用
    fn process_entry(e: &std::fs::DirEntry, pb: &ProgressBar, parallel: bool) -> u64 {
        match e.metadata() {
            Ok(metadata) => {
                if metadata.is_dir() {
                    inner_calculate(&e.path(), pb, parallel)
                } else {
                    metadata.len()
                }
            }
            Err(e) => {
                eprintln!("无法获取文件元数据 {}", e);
                0 // 返回0表示这个文件无法访问，但不影响目录计算其他项
            }
        }
    }

    main_pb.set_message(format!("计算 {}...", path.display()));
    let total = inner_calculate(path, main_pb, parallel);
    // println!("Total size: {}", total);
    main_pb.set_message("处理中...");

    let converted = if human_readable {
        human_readable_size(total)
    } else {
        total.to_string()
    };
    (total, converted)
}

pub fn list_directory(path: &Path, args: &Cli) {
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
        let pb_arc = Arc::new(&process_pb);
        for (_i, file) in files.iter().enumerate() {
            process_pb.tick();
            let file_path = path.join(&file);
            if args.name.is_some() {
                let metadata = match file_path.metadata() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("ls: cannot access '{}': {}", file_path.display(), e);
                        continue;
                    }
                };
                if metadata.is_dir() {
                    // 如果是目录，是否跟要搜索的名称匹配
                    if let Some(name) = &args.name {
                        if !file.contains(name) {
                            // 使用并行版本
                            calculate_dir_size_parallel(
                                file_path,
                                args.human_readable,
                                Arc::clone(&pb_arc), // 克隆 Arc
                                name,
                                &mut entries,
                            );
                            continue;
                        }
                    }
                } else {
                    continue;
                }
            }
            let metadata = match file_path.metadata() {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("ls: cannot access '{}': {}", file_path.display(), e);
                    continue;
                }
            };
            let (size_display, size_raw) = if metadata.is_dir() {
                let (raw, converted) =
                    calculate_dir_size(&file_path, args.human_readable, &process_pb, args.parallel);
                (converted, raw)
            } else if args.human_readable {
                (human_readable_size(metadata.len()), metadata.len())
            } else {
                (metadata.len().to_string(), metadata.len())
            };
            entries.push(FileEntry {
                file_type: if metadata.is_dir() { 'd' } else { '-' },
                permissions: format!(
                    "{}-{}-{}",
                    if metadata.permissions().readonly() {
                        "r"
                    } else {
                        " "
                    },
                    "w",
                    "x"
                ),
                size_display,
                size_raw,
                path: match file_path.canonicalize() {
                    Ok(canonical_path) => get_canonical_path(&canonical_path),
                    Err(_e) => {
                        // eprintln!("获取绝对路径失败: {}", e);
                        file_path.to_string_lossy().into_owned()
                    }
                },
            });
        }

        process_pb.finish_and_clear();
        let mut sum_size = 0;
        for entry in &entries {
            sum_size += entry.size_raw; // 使用第4个字段的原始大小
        }
        if args.sort {
            entries.sort_by(|a, b| a.size_raw.cmp(&b.size_raw));
        }

        let mut table = Table::new();
        table
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("类型").add_attribute(comfy_table::Attribute::Bold),
                Cell::new("权限").add_attribute(comfy_table::Attribute::Bold),
                Cell::new("大小").add_attribute(comfy_table::Attribute::Bold),
                Cell::new("路径").add_attribute(comfy_table::Attribute::Bold),
            ])
            .load_preset(comfy_table::presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);

        for entry in entries.iter() {
            let file_path = if args.full_path {
                &entry.path
            } else {
                entry
                    .path
                    .split(MAIN_SEPARATOR)
                    .last()
                    .unwrap_or(&entry.path)
            };
            table.add_row(vec![
                Cell::new(&entry.file_type.to_string())
                    .set_alignment(comfy_table::CellAlignment::Center),
                Cell::new(entry.permissions.replace('-', "")),
                Cell::new(&entry.size_display),
                Cell::new(file_path),
            ]);
        }

        println!("{}", table);
        println!("┌{:─^33}┐", "");
        println!(
            "│ 总数量: {:6} │ 总大小: {:10} ",
            entries.len(),
            human_readable_size(sum_size)
        );
        println!("└{:─^33}┘", "");
    } else {
        for file in files {
            println!("{}", file);
        }
    }
    scan_pb.finish_and_clear(); // 完成后清理进度条
}

// 搜索文件
fn calculate_dir_size_parallel(
    file_path: PathBuf,
    human_readable: bool,
    pb: Arc<&ProgressBar>, // 改为 Arc
    name: &str,
    entries: &mut Vec<FileEntry>,
) {
    let sub_entries = match fs::read_dir(&file_path) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("ls: cannot access '{}': {}", file_path.display(), e);
            return;
        }
    };

    // 收集所有需要处理的目录
    let dirs_to_process: Vec<_> = sub_entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                return None;
            }
            let metadata = e.metadata().ok()?;
            if !metadata.is_dir() {
                return None;
            }
            Some((e.path(), name))
        })
        .collect();

    // 并行处理每个子目录
    let results: Vec<Vec<FileEntry>> = dirs_to_process
        .into_par_iter()
        .map(|(sub_path, sub_name)| {
            pb.tick();
            let mut local_entries = Vec::new();

            if sub_name.contains(name) {
                // 匹配：计算大小
                let (raw, converted) = calculate_dir_size(&sub_path, human_readable, &pb, true);
                local_entries.push(FileEntry {
                    file_type: 'd',
                    permissions: "rwx".to_string(),
                    size_display: converted,
                    size_raw: raw,
                    path: get_canonical_path(&sub_path),
                });
            } else {
                calculate_dir_size_parallel(
                    sub_path,
                    human_readable,
                    Arc::clone(&pb),
                    name,
                    &mut local_entries,
                );
            }
            local_entries
        })
        .collect();

    // 收集所有结果到主entries
    for result in results {
        entries.extend(result);
    }
}

fn get_canonical_path(path: &Path) -> String {
    match path.canonicalize() {
        Ok(canonical) => {
            let s = canonical.to_string_lossy().into_owned();
            s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
        }
        Err(_) => path.to_string_lossy().into_owned(),
    }
}
