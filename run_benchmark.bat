@echo off
REM 创建基准测试报告目录
if not exist benchmark_results mkdir benchmark_results

REM 运行基准测试并保存结果
echo 运行基准测试...
cargo bench --bench my_benchmark > benchmark_results/benchmark_output.txt 2>&1

REM 生成HTML报告
echo 生成HTML报告...
cargo bench --bench my_benchmark -- --output-format html > benchmark_results/benchmark_report.html 2>&1

REM 生成图表
echo 生成图表...
cargo bench --bench my_benchmark -- --output-format plotters > benchmark_results/benchmark_plots.html 2>&1

echo 基准测试完成！结果保存在 benchmark_results 目录中。
pause
