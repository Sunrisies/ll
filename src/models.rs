use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    version,
    author,
    about = "一个类似 ls 的命令行目录列表工具",
    long_about = "用法示例:\n  ll -l 查看详细列表\n  ll -a 显示隐藏文件"
)]
pub struct Cli {
    /// 指定要列出的文件或目录
    #[arg(default_value = ".", value_name = "FILE")]
    pub file: String,

    /// 启用详细模式
    #[arg(short = 'l', long = "long", help = "使用长列表格式")]
    pub long_format: bool,

    /// 启用人类可读的文件大小
    #[arg(
        short = 'H',
        long = "human-readable",
        help = "使用易读的文件大小格式 (例如 1K, 234M, 2G)"
    )] // 修改短选项为 -H
    pub human_readable: bool,

    /// 显示隐藏文件
    #[arg(short = 'a', long = "all", help = "不要忽略以开头的条目 .")]
    pub all: bool,

    /// 显示程序运行时间
    #[arg(short = 't', long = "time")]
    pub show_time: bool,

    /// 启用并行处理加速扫描
    #[arg(short = 'f', long = "fast")]
    pub parallel: bool,

    #[arg(short = 's', long = "sort")]
    pub sort: bool,

    /// 搜索文件名或目录名
    #[arg(long = "name", value_name = "PATTERN")]
    pub name: Option<String>,
}

pub struct FileEntry {
    pub file_type: char,
    pub permissions: String,
    pub size_display: String,
    pub size_raw: u64,
    pub path: String,
}
