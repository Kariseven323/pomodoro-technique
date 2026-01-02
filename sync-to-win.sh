#!/usr/bin/env bash
# 同步代码到 Windows 目录（保留 Windows 端构建缓存；其余内容删除并覆盖同步）。
#
# 设计目标：
# - Windows 端保留构建缓存（默认：`node_modules/` 与 `src-tauri/target/`），避免每次打包全量重建。
# - 其它文件保持“删了再覆盖”的效果：同步时删除 Windows 端多余文件，并用本机（WSL）代码覆盖。
# - 同步集合遵循 `.gitignore`：忽略项默认不从 WSL 端同步，但会在 Windows 端被删除（除非被保护）。

set -euo pipefail

WIN_PATH_DEFAULT="/mnt/c/Users/KAR1SEVEN/Desktop/projects/pomodoro-technique"
WIN_PATH="${WIN_PATH:-$WIN_PATH_DEFAULT}"

SYNC_DRY_RUN="${SYNC_DRY_RUN:-0}"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

## 清理临时文件（由 EXIT trap 调用）。
cleanup() {
  :
}

## 获取 git 仓库根目录；若不可用则回退到脚本所在目录。
get_repo_root() {
  if git -C "$SCRIPT_DIR" rev-parse --show-toplevel >/dev/null 2>&1; then
    git -C "$SCRIPT_DIR" rev-parse --show-toplevel
    return 0
  fi
  echo "$SCRIPT_DIR"
}

## 构建 rsync 参数：保护 Windows 端缓存目录，其余内容按 `.gitignore` 删除并覆盖。
## 执行 rsync 同步：保留缓存目录，其余删除并覆盖。
run_sync() {
  local repo_root="$1"
  local dest_dir="$2"

  local -a rsync_args
  rsync_args=(-a -v --delete --delete-excluded --modify-window=2)

  # 不同步 .git，同时保护 Windows 端可能存在的 .git（避免误删）。
  rsync_args+=(--filter="P .git/")
  rsync_args+=(--exclude=".git/")

  # 保护并跳过同步 Windows 端构建缓存（避免每次重新打包）。
  rsync_args+=(--filter="P /node_modules/")
  rsync_args+=(--exclude="/node_modules/")
  rsync_args+=(--filter="P /src-tauri/target/")
  rsync_args+=(--exclude="/src-tauri/target/")

  # 同步集合遵循 .gitignore；但被忽略的文件会在 Windows 端被删除（因为启用了 --delete-excluded）。
  rsync_args+=(--filter=":- .gitignore")

  if [[ "$SYNC_DRY_RUN" == "1" ]]; then
    rsync_args+=(-n)
  fi

  rsync "${rsync_args[@]}" "$repo_root"/ "$dest_dir"/
}

## 主入口：解析参数并执行同步。
main() {
  local repo_root
  repo_root="$(get_repo_root)"

  if [[ ! -d "$repo_root" ]]; then
    echo "仓库根目录不存在：$repo_root" >&2
    return 1
  fi

  if ! command -v rsync >/dev/null 2>&1; then
    echo "未找到 rsync，请先安装 rsync 后再运行。" >&2
    return 1
  fi

  mkdir -p "$WIN_PATH"

  trap cleanup EXIT

  run_sync "$repo_root" "$WIN_PATH"

  echo "已同步到 $WIN_PATH（repo_root=$repo_root, dry_run=$SYNC_DRY_RUN；保留 node_modules/ 与 src-tauri/target/）"
}

main "$@"
