#!/usr/bin/env bash
# Bulk-fetch the pinned harvest corpus (see gutenberg-100.txt).
# Public-domain texts to OUT_DIR (default /tmp/opencode/books100);
# never vendored. Polite: 2s between requests, one retry, three
# filename variants per ID (`N-0.txt` UTF-8 preferred, then `-8`,
# then bare). Writes MANIFEST.tsv (id, file, bytes, md5, variant)
# so any run is reproducible and gaps are visible.
#
# Usage: scripts/fetch-books.sh [--dir DIR] [--sleep SECONDS]
set -euo pipefail

DIR="/tmp/opencode/books100"
SLEEP="2"
while [[ $# -gt 0 ]]; do
  case "$1" in
  --dir)
    DIR="$2"
    shift 2
    ;;
  --sleep)
    SLEEP="$2"
    shift 2
    ;;
  *)
    echo "usage: fetch-books.sh [--dir DIR] [--sleep SECONDS]" >&2
    exit 2
    ;;
  esac
done

LIST="$(dirname "$0")/gutenberg-100.txt"
mkdir -p "$DIR"
MANIFEST="$DIR/MANIFEST.tsv"
: >"$MANIFEST"

count=0
while read -r id _rest; do
  case "$id" in
  "" | \#*) continue ;;
  esac
  for variant in "$id-0.txt" "$id-8.txt" "$id.txt"; do
    out="$DIR/$id.txt"
    if [[ ! -s $out ]]; then
      if curl -fSL --max-time 120 -o "$out" \
        "https://www.gutenberg.org/files/$id/$variant" 2>/dev/null; then
        [[ -s $out ]] || continue
      else
        rm -f "$out"
        continue
      fi
    fi
    bytes=$(wc -c <"$out")
    md5=$(md5sum "$out" | cut -d' ' -f1)
    printf '%s\t%s\t%s\t%s\t%s\n' "$id" "$id.txt" "$bytes" "$md5" "$variant" >>"$MANIFEST"
    count=$((count + 1))
    break
  done
  if [[ ! -s $DIR/$id.txt ]]; then
    printf '%s\t%s\t%s\t%s\t%s\n' "$id" "MISSING" "0" "-" "-" >>"$MANIFEST"
    echo "MISSING: $id" >&2
  fi
  sleep "$SLEEP"
done < <(awk '/^[0-9]+ /{print $1}' "$LIST")
echo "fetched $count texts -> $DIR"
