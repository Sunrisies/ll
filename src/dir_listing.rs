use super::models::{Cli, FileEntry};
use super::utils::{human_readable_size, progress_bar_init};
use comfy_table::{Cell, ContentArrangement, Table};
use indicatif::ProgressBar;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use std::sync::Arc;

/// 递归计算目录总大小（无进度条内部开销）
fn calc_dir_size_inner(path: &Path, parallel: bool, depth: usize) -> u64 {
    let Ok(entries) = fs::read_dir(path) else { return 0 };

    let items: Vec<_> = entries
        .flatten()
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            Some((e.path(), meta))
        })
        .collect();

    let use_parallel = parallel && depth < 8 && items.len() > 4;

    if use_parallel {
        items
            .into_par_iter()
            .map(|(p, meta)| {
                if meta.is_dir() {
                    calc_dir_size_inner(&p, true, depth + 1)
                } else {
                    meta.len()
                }
            })
            .sum()
    } else {
        let mut total = 0u64;
        for (p, meta) in &items {
            if meta.is_dir() {
                total += calc_dir_size_inner(p, parallel && depth < 10, depth + 1);
            } else {
                total += meta.len();
            }
        }
        total
    }
}

pub fn calculate_dir_size(
    path: &Path,
    human_readable: bool,
    main_pb: &ProgressBar,
    parallel: bool,
) -> (u64, String) {
    main_pb.set_message(format!("计算 {}...", path.display()));
    let total = calc_dir_size_inner(path, parallel, 0);
    let converted = if human_readable {
        human_readable_size(total)
    } else {
        total.to_string()
    };
    (total, converted)
}

/// 根据元数据直接构建 FileEntry（目录会递归算大小）
fn build_file_entry_from_meta(
    name: &str,
    meta: &fs::Metadata,
    base_path: &Path,
    human_readable: bool,
    parallel: bool,
) -> FileEntry {
    let file_path = base_path.join(name);
    let (size_display, size_raw) = if meta.is_dir() {
        let total = calc_dir_size_inner(&file_path, parallel, 0);
        let converted = if human_readable {
            human_readable_size(total)
        } else {
            total.to_string()
        };
        (converted, total)
    } else if human_readable {
        (human_readable_size(meta.len()), meta.len())
    } else {
        (meta.len().to_string(), meta.len())
    };
    FileEntry {
        file_type: if meta.is_dir() { 'd' } else { '-' },
        permissions: format!(
            "{}-{}-{}",
            if meta.permissions().readonly() { "r" } else { " " },
            "w",
            "x"
        ),
        size_display,
        size_raw,
        path: match file_path.canonicalize() {
            Ok(canonical_path) => get_canonical_path(&canonical_path),
            Err(_e) => file_path.to_string_lossy().into_owned(),
        },
    }
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

    // --all 控制是否显示隐藏文件
    if !args.all {
        files.retain(|f| !f.starts_with('.'));
    }

    // --name: 递归搜索匹配名称的目录（类似 npkill）
    if let Some(pattern) = &args.name {
        let pb = progress_bar_init(None).unwrap();
        pb.set_message("搜索中...");

        let mut name_results: Vec<FileEntry> = Vec::new();
        search_matching_dirs(
            path,
            pattern,
            args.human_readable,
            &pb,
            args.parallel,
            args.all,
            &mut name_results,
        );
        pb.finish_and_clear();

        if name_results.is_empty() {
            println!("未找到包含 '{}' 的目录", pattern);
            return;
        }

        if args.sort {
            name_results.sort_by(|a, b| a.size_raw.cmp(&b.size_raw));
        }

        if args.long_format {
            let mut table = Table::new();
            table
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_header(vec![
                    Cell::new("类型").add_attribute(comfy_table::Attribute::Bold),
                    Cell::new("大小").add_attribute(comfy_table::Attribute::Bold),
                    Cell::new("路径").add_attribute(comfy_table::Attribute::Bold),
                ])
                .load_preset(comfy_table::presets::UTF8_FULL)
                .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);

            let mut sum_size = 0;
            for entry in &name_results {
                sum_size += entry.size_raw;
                table.add_row(vec![
                    Cell::new("d").set_alignment(comfy_table::CellAlignment::Center),
                    Cell::new(&entry.size_display),
                    Cell::new(&entry.path),
                ]);
            }

            println!("{}", table);
            println!("┌{:─^43}┐", "");
            println!("│ 匹配目录: {:4} │ 总大小: {:10} │", name_results.len(), human_readable_size(sum_size));
            println!("└{:─^43}┘", "");
        } else {
            for entry in &name_results {
                println!("{}", entry.path);
            }
        }
        return;
    }

    let scan_pb = progress_bar_init(None).unwrap();

    if args.long_format {
        let process_pb = progress_bar_init(None).unwrap();
        process_pb.set_message("扫描元数据...");

        // Phase 1: 快速收集所有条目的元数据（stat 操作，轻量）
        let mut meta_list: Vec<(String, fs::Metadata)> = Vec::new();
        for file in &files {
            process_pb.tick();
            let file_path = path.join(file);
            match file_path.metadata() {
                Ok(m) => meta_list.push((file.clone(), m)),
                Err(e) => eprintln!("ls: cannot access '{}': {}", file_path.display(), e),
            }
        }

        // Phase 2: 构建 FileEntry（目录算大小可并行）
        process_pb.set_message("计算大小...");
        let pb_arc = Arc::new(process_pb.clone());
        let entries: Vec<FileEntry> = if args.parallel && meta_list.len() > 1 {
            meta_list
                .par_iter()
                .map(|(name, meta)| {
                    pb_arc.tick();
                    build_file_entry_from_meta(name, meta, path, args.human_readable, true)
                })
                .collect()
        } else {
            meta_list
                .iter()
                .map(|(name, meta)| {
                    process_pb.tick();
                    build_file_entry_from_meta(name, meta, path, args.human_readable, args.parallel)
                })
                .collect()
        };

        process_pb.finish_and_clear();

        let mut sum_size = 0u64;
        for entry in &entries {
            sum_size += entry.size_raw;
        }
        let mut entries = entries;
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
            let display_path = if args.full_path {
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
                Cell::new(display_path),
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

/// 收集目录下的所有可见子目录
fn collect_subdirs(path: &Path, show_all: bool) -> Vec<(PathBuf, String)> {
    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if !show_all && name.starts_with('.') {
                return None;
            }
            let metadata = entry.metadata().ok()?;
            if !metadata.is_dir() {
                return None;
            }
            Some((entry.path(), name))
        })
        .collect()
}

fn search_matching_dirs(
    path: &Path,
    pattern: &str,
    human_readable: bool,
    pb: &ProgressBar,
    parallel: bool,
    show_all: bool,
    results: &mut Vec<FileEntry>,
) {
    let dirs = collect_subdirs(path, show_all);
    if dirs.is_empty() {
        return;
    }

    // 分开匹配和不匹配的目录
    let mut matching: Vec<(PathBuf, String)> = Vec::new();
    let mut non_matching: Vec<PathBuf> = Vec::new();
    for (p, name) in dirs {
        if name.contains(pattern) {
            matching.push((p, name));
        } else {
            non_matching.push(p);
        }
    }

    // Phase 1: 匹配的目录 → 并行计算大小
    if !matching.is_empty() {
        let pb_arc = Arc::new(pb.clone());
        let match_results: Vec<FileEntry> = if parallel && matching.len() > 1 {
            matching
                .par_iter()
                .map(|(dir_path, _)| {
                    pb_arc.tick();
                    pb_arc.set_message(format!("找到: {}", dir_path.display()));
                    let (raw, converted) =
                        calculate_dir_size(dir_path, human_readable, &pb_arc, true);
                    FileEntry {
                        file_type: 'd',
                        permissions: "rwx".to_string(),
                        size_display: converted,
                        size_raw: raw,
                        path: get_canonical_path(dir_path),
                    }
                })
                .collect()
        } else {
            matching
                .iter()
                .map(|(dir_path, _)| {
                    pb.tick();
                    pb.set_message(format!("找到: {}", dir_path.display()));
                    let (raw, converted) =
                        calculate_dir_size(dir_path, human_readable, pb, parallel);
                    FileEntry {
                        file_type: 'd',
                        permissions: "rwx".to_string(),
                        size_display: converted,
                        size_raw: raw,
                        path: get_canonical_path(dir_path),
                    }
                })
                .collect()
        };
        results.extend(match_results);
    }

    // Phase 2: 不匹配的目录 → 递归搜索（并行）
    if non_matching.is_empty() {
        return;
    }

    if parallel && non_matching.len() > 1 {
        let sub_results: Vec<Vec<FileEntry>> = non_matching
            .par_iter()
            .map(|dir_path| {
                let mut local = Vec::new();
                search_dirs_deep(dir_path, pattern, human_readable, show_all, &mut local);
                local
            })
            .collect();
        for r in sub_results {
            results.extend(r);
        }
    } else {
        for dir_path in &non_matching {
            search_matching_dirs(dir_path, pattern, human_readable, pb, parallel, show_all, results);
        }
    }
}

/// 无进度条版搜索，专用于并行递归（每个线程独立跑，不抢 pb）
fn search_dirs_deep(
    path: &Path,
    pattern: &str,
    human_readable: bool,
    show_all: bool,
    results: &mut Vec<FileEntry>,
) {
    let dirs = collect_subdirs(path, show_all);
    if dirs.is_empty() {
        return;
    }

    // 本层匹配的 → 计算大小
    let mut deeper: Vec<PathBuf> = Vec::new();
    for (dir_path, name) in &dirs {
        if name.contains(pattern) {
            // 不用 pb，创建临时 spinner 给 calculate_dir_size 占位
            let silent_pb = ProgressBar::new_spinner();
            let (raw, converted) =
                calculate_dir_size(dir_path, human_readable, &silent_pb, true);
            results.push(FileEntry {
                file_type: 'd',
                permissions: "rwx".to_string(),
                size_display: converted,
                size_raw: raw,
                path: get_canonical_path(dir_path),
            });
        } else {
            deeper.push(dir_path.clone());
        }
    }

    // 不匹配的 → 继续递归（并行）
    if deeper.len() > 1 {
        let sub: Vec<Vec<FileEntry>> = deeper
            .par_iter()
            .map(|p| {
                let mut local = Vec::new();
                search_dirs_deep(p, pattern, human_readable, show_all, &mut local);
                local
            })
            .collect();
        for r in sub {
            results.extend(r);
        }
    } else {
        for p in &deeper {
            search_dirs_deep(p, pattern, human_readable, show_all, results);
        }
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
