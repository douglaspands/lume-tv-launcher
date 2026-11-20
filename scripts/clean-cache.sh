#!/usr/bin/env bash
# Creates (with Btrfs NOCOW / chattr +C) and cleans the repo's local ./cache directory.
#
# Usage:
#   scripts/clean-cache.sh           Clean ./cache and recreate it with NOCOW (+C)
#   scripts/clean-cache.sh --init    Ensure ./cache exists with NOCOW (+C) without deleting files
#   scripts/clean-cache.sh --all     Also remove legacy ./target, ./var, and scripts/__pycache__
#   scripts/clean-cache.sh --status  Show ./cache size and NOCOW attribute status

set -euo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
cache_dir="$repo/cache"

apply_nocow() {
    local dir="$1"
    if command -v chattr >/dev/null 2>&1; then
        if chattr +C "$dir" 2>/dev/null; then
            echo "NOCOW (+C) applied to $dir"
        else
            echo "note: could not apply chattr +C to $dir (non-Btrfs or unsupported filesystem)"
        fi
    else
        echo "note: chattr not found; skipping NOCOW (+C)"
    fi
}

ensure_subdirs() {
    mkdir -p "$cache_dir/target" "$cache_dir/tmp"
}

show_status() {
    if [[ ! -d "$cache_dir" ]]; then
        echo "./cache does not exist (run scripts/clean-cache.sh --init)"
        return 0
    fi
    du -sh "$cache_dir"
    if command -v lsattr >/dev/null 2>&1; then
        lsattr -d "$cache_dir" 2>/dev/null || true
    fi
}

mode="${1:-clean}"

case "$mode" in
    --status)
        show_status
        ;;
    --init)
        if [[ ! -d "$cache_dir" ]]; then
            mkdir -p "$cache_dir"
            apply_nocow "$cache_dir"
        fi
        ensure_subdirs
        show_status
        ;;
    clean|--all)
        if [[ -d "$cache_dir" ]]; then
            before="$(du -sh "$cache_dir" | cut -f1)"
            echo "Cleaning $cache_dir (currently $before)..."
            rm -rf "$cache_dir"
        fi
        mkdir -p "$cache_dir"
        apply_nocow "$cache_dir"
        ensure_subdirs
        if [[ "$mode" == "--all" ]]; then
            for legacy in "$repo/target" "$repo/var" "$repo/scripts/__pycache__"; do
                if [[ -e "$legacy" ]]; then
                    echo "Removing legacy $legacy..."
                    rm -rf "$legacy"
                fi
            done
        fi
        show_status
        ;;
    -h|--help)
        sed -n '2,9p' "$0" | sed 's/^# \?//'
        ;;
    *)
        echo "Unknown option: $mode (use --init, --all, --status, or --help)" >&2
        exit 1
        ;;
esac
