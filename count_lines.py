#!/usr/bin/env python3
"""统计各目录下代码文件行数，按行数降序排列"""

import os
from pathlib import Path
from collections import defaultdict

# 代码文件扩展名
EXTENSIONS = {'.svelte', '.ts', '.rs', '.js', '.css', '.html', '.json'}
# 忽略目录
IGNORE_DIRS = {'node_modules', 'target', 'dist', '.svelte-kit', '.git', 'build'}

def count_lines(file_path):
    """统计文件行数"""
    try:
        with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
            return sum(1 for _ in f)
    except:
        return 0

def main():
    root = Path('.')
    files = []

    for path in root.rglob('*'):
        if path.is_file() and path.suffix in EXTENSIONS:
            if any(d in path.parts for d in IGNORE_DIRS):
                continue
            lines = count_lines(path)
            files.append((str(path), lines))

    # 按行数降序排序
    files.sort(key=lambda x: -x[1])

    # 按目录分组统计
    dir_totals = defaultdict(int)
    for path, lines in files:
        dir_name = str(Path(path).parent)
        dir_totals[dir_name] += lines

    print("=" * 60)
    print("文件行数统计（降序）")
    print("=" * 60)
    for path, lines in files:
        print(f"{lines:>6} 行  {path}")

    print("\n" + "=" * 60)
    print("目录汇总（降序）")
    print("=" * 60)
    for dir_name, total in sorted(dir_totals.items(), key=lambda x: -x[1]):
        print(f"{total:>6} 行  {dir_name}/")

    print(f"\n总计: {sum(l for _, l in files)} 行")

if __name__ == '__main__':
    main()
