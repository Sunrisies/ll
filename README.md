# ll - 高性能目录列表工具

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org/)

> 一个快速、强大的目录列表工具，提供详细的文件信息和灵活的过滤选项

## ✨ 特性

- ⚡ **极速并行扫描** - 采用 Rust + rayon 并行处理，比传统 ls 快 3-5 倍
- 📊 **详细的表格化输出** - 清晰展示文件类型、权限、大小和路径信息
- 🔍 **递归名称搜索** - 类似 npkill，递归搜索匹配名称的目录，自动计算大小
- 📈 **智能并行策略** - 根据目录复杂度自动选择最优并行方式
- 💾 **人类可读格式** - 自动转换文件大小为 KB、MB、GB 等易读单位
- 🎯 **灵活的排序** - 支持按文件大小排序
- 🚀 **高性能设计** - 动态并行策略、批量进度更新、优化的内存使用
- 📏 **完整路径支持** - 可选显示完整的规范化路径

## 📦 安装

### 从源码安装

```bash
# 克隆仓库
git clone https://github.com/Sunrisies/ll.git
cd ll

# 构建 release 版本
cargo build --release

# 二进制文件位于 target/release/ll
```

### 使用 Cargo 安装

```bash
cargo install --path .
```

## 🚀 快速开始

### 基本用法

```bash
# 列出当前目录（简单模式）
ll

# 详细列表模式
ll -l

# 人类可读的文件大小
ll -lH

# 显示所有文件（包括隐藏文件）
ll -a

# 组合使用：详细 + 可读大小 + 隐藏文件
ll -lHa
```

### 高级用法

```bash
# 快速并行模式（大目录推荐）
ll -lHf

# 按文件大小排序
ll -lHs

# 显示完整路径
ll -lHp

# 递归搜索 node_modules（类似 npkill）
ll -lH --name "node_modules"

# 递归搜索匹配名称的目录
ll -lH --name "src"

# 显示运行时间（性能分析）
ll -lHt
```

## 📖 命令参数

| 参数 | 长参数 | 说明 |
|------|--------|------|
| `-l` | `--long` | 使用长列表格式，显示详细信息 |
| `-H` | `--human-readable` | 使用易读的文件大小格式（如 1K, 234M, 2G） |
| `-a` | `--all` | 显示隐藏文件（以 . 开头的文件） |
| `-f` | `--fast` | 启用并行处理加速扫描（大目录推荐） |
| `-s` | `--sort` | 按文件大小排序 |
| `-t` | `--time` | 显示程序运行时间 |
| `-p` | `--full-path` | 显示完整路径 |
| | `--name <模式>` | 递归搜索匹配名称的目录，显示大小和路径 |

## 🎯 使用场景

### 日常使用

```bash
# 快速查看当前目录内容
ll -lH

# 查找大文件
ll -lHs

# 分析项目结构
ll -lHf /path/to/project
```

### 开发场景

```bash
# 递归查找 node_modules 目录
ll -lH --name "node_modules"

# 递归查找 test 目录
ll -lH --name "test"

# 递归查找 config 目录（短格式，仅显示路径）
ll --name "config"
```

### 性能分析

```bash
# 测试扫描性能
ll -lHft large_directory

# 对比串行和并行性能
ll -lHt     # 不使用并行
ll -lHft    # 使用并行
```

## 📊 性能对比

在典型场景下的性能表现：

| 目录大小 | 文件数 | ll (并行) | GNU ls | 性能提升 |
|---------|-------|----------|--------|---------|
| 小型 | ~100 | 0.01s | 0.02s | **2x** |
| 中型 | ~1000 | 0.15s | 0.45s | **3x** |
| 大型 | ~10000 | 1.2s | 5.6s | **4.7x** |

*测试环境: Intel i7-8核, SSD, Windows 11*

### 性能优势

- ✅ **并行处理**: 使用 rayon 实现智能并行策略
- ✅ **动态优化**: 根据目录深度和文件数自动调整并行度
- ✅ **优化编译**: LTO、strip、opt-level=3 全面优化
- ✅ **进度控制**: 批量更新减少UI开销
- ✅ **内存高效**: 优化的数据结构和内存分配策略

## 🔧 配置与扩展

### 性能提示

- 💡 大目录（>1000 文件）建议使用 `-f` 并行模式
- 💡 网络驱动器上避免使用并行模式（可能适得其反）
- 💡 不需要隐藏文件时省略 `-a` 可提升性能
- 💡 使用 `-t` 参数测量实际性能

### 输出示例

```bash
$ ll -lH
┌──────┬────────┬─────────┬────────────────────────────────┐
│ 类型 │  权限  │  大小   │              路径               │
├──────┼────────┼─────────┼────────────────────────────────┤
│  d   │  rwx   │  4.0KB  │ src                            │
│  -   │  rwx   │  1.0KB  │ Cargo.toml                     │
│  -   │  rwx   │  35.5KB │ Cargo.lock                     │
│  d   │  rwx   │  4.0KB  │ benches                        │
└──────┴────────┴─────────┴────────────────────────────────┘
┌─────────────────────────────────┐
│ 总数量:      4 │ 总大小:  44.5KB  
└─────────────────────────────────┘
```

## 🛠️ 开发

### 构建

```bash
# 开发构建
cargo build

# Release 构建（推荐）
cargo build --release

# 运行测试
cargo test

# 性能基准测试
cargo bench
```

### 项目结构

```
ll/
├── src/
│   ├── main.rs          # CLI 入口
│   ├── lib.rs           # 库接口
│   ├── models.rs        # 数据模型
│   ├── dir_listing.rs   # 核心逻辑（目录扫描、大小计算）
│   ├── utils.rs         # 工具函数（格式化、进度条）
│   └── log.rs           # 日志系统
├── benches/
│   └── my_benchmark.rs  # 性能基准测试
├── Cargo.toml           # 项目配置
└── README.md            # 本文档
```

### 技术栈

- **语言**: Rust 2021 Edition
- **并行处理**: rayon 1.10
- **命令行解析**: clap 4.5
- **表格渲染**: comfy-table 7.1
- **进度显示**: indicatif 0.17
- **日志系统**: log4rs 1.4
- **目录遍历**: jwalk 0.8（计划集成）

## 🤝 贡献

欢迎贡献代码、报告问题或提出建议！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 📝 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情

## 🙏 致谢

- [exa](https://github.com/ogham/exa) - 现代化的 ls 替代品，提供了设计灵感
- [fd](https://github.com/sharkdp/fd) - 快速的文件查找工具
- [ripgrep](https://github.com/BurntSushi/ripgrep) - 极速的文本搜索工具
- Rust 社区提供的优秀库和工具

## 📬 联系方式

- 作者: 朝阳
- Email: 3266420686@qq.com
- 仓库: https://github.com/Sunrisies/ll

---

**⭐ 如果这个项目对你有帮助，请给个 Star！**
