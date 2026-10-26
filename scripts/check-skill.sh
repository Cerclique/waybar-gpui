#!/usr/bin/env bash
# Checks the authors' agent skill (skills/waybar-gpui/SKILL.md): every doc it
# names (`docs/guide/<page>.md`, `docs/GLOSSARY.md`) exists. The skill names
# them as paths inside a versioned URL, so lychee can't follow them. Used by
# CI; run it locally from anywhere.
set -euo pipefail
cd "$(dirname "$0")/.."

skill=skills/waybar-gpui/SKILL.md
missing=$(grep -oE 'docs/[A-Za-z/_-]+\.md' "$skill" | sort -u |
  while read -r doc; do [[ -f "$doc" ]] || echo "$doc"; done)
if [[ -n "$missing" ]]; then
  echo "$skill names docs that don't exist:" >&2
  echo "$missing" >&2
  exit 1
fi
echo "every doc $skill names exists"
