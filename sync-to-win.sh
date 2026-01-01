#!/usr/bin/env bash
# 同步代码到 Windows 目录（默认排除 .gitignore 中的文件；并保证从仓库根目录同步）。

set -euo pipefail

WIN_PATH_DEFAULT="/mnt/c/Users/KAR1SEVEN/Desktop/projects/pomodoro-technique"
WIN_PATH="${WIN_PATH:-$WIN_PATH_DEFAULT}"

SYNC_INCLUDE_IGNORED="${SYNC_INCLUDE_IGNORED:-0}"
SYNC_DRY_RUN="${SYNC_DRY_RUN:-0}"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

TMP_LIST=""

## 清理临时文件（由 EXIT trap 调用）。
cleanup() {
  if [[ -n "${TMP_LIST:-}" ]]; then
    rm -f -- "$TMP_LIST"
  fi
}

## 获取 git 仓库根目录；若不可用则回退到脚本所在目录。
get_repo_root() {
  if git -C "$SCRIPT_DIR" rev-parse --show-toplevel >/dev/null 2>&1; then
    git -C "$SCRIPT_DIR" rev-parse --show-toplevel
    return 0
  fi
  echo "$SCRIPT_DIR"
}

## 删除目标目录下的全部内容（包含以 '.' 开头的隐藏文件/目录）。
clear_destination_dir() {
  local dest_dir="$1"
  if [[ ! -d "$dest_dir" ]]; then
    return 0
  fi
  find "$dest_dir" -mindepth 1 -maxdepth 1 -exec rm -rf -- {} +
}

## 生成需要同步的文件清单（NUL 分隔，供 rsync --from0 使用）。
generate_file_list() {
  local repo_root="$1"
  local list_file="$2"

  if [[ "$SYNC_INCLUDE_IGNORED" == "1" ]]; then
    {
      git -C "$repo_root" ls-files -z
      git -C "$repo_root" ls-files -z --others --ignored --exclude-standard
    } >"$list_file"
    return 0
  fi

  {
    git -C "$repo_root" ls-files -z
    git -C "$repo_root" ls-files -z --others --exclude-standard
  } >"$list_file"
}

## 执行 rsync 同步（基于 files-from 的精确文件集同步）。
run_rsync_sync() {
  local repo_root="$1"
  local dest_dir="$2"
  local list_file="$3"

  local -a rsync_args
  rsync_args=(-a -v --from0 --files-from="$list_file")

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

  TMP_LIST="$(mktemp -t pomodoro-sync-files.XXXXXX)"
  generate_file_list "$repo_root" "$TMP_LIST"

  if [[ "$SYNC_DRY_RUN" != "1" ]]; then
    clear_destination_dir "$WIN_PATH"
  fi

  run_rsync_sync "$repo_root" "$WIN_PATH" "$TMP_LIST"

  echo "已同步到 $WIN_PATH（repo_root=$repo_root, include_ignored=$SYNC_INCLUDE_IGNORED, dry_run=$SYNC_DRY_RUN）"
}

main "$@"
