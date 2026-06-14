#!/usr/bin/env bash
# Проставляет одну версию во все манифесты проекта (сервер + десктоп).
# Используется CI при выпуске релиза, но запускается и локально:
#   ./scripts/set-version.sh 2.1.0   (ведущая "v" в теге отбрасывается)
set -euo pipefail

raw="${1:?usage: set-version.sh <version>}"
version="${raw#v}"

root="$(cd "$(dirname "$0")/.." && pwd)"

# --- Rust-манифесты (TOML): меняем только первое поле version, т.е. [package].
set_cargo_version() {
  local file="$1"
  perl -i -pe 'if (!$seen && /^\s*version\s*=/) { s/=.*/= "'"$version"'"/; $seen = 1 }' "$file"
}

# --- JSON-манифесты: правим через node (есть на всех runner'ах), отступ сохраняем.
set_json_version() {
  local file="$1"
  node -e 'const fs=require("fs");const p=process.argv[1];const j=JSON.parse(fs.readFileSync(p,"utf8"));j.version=process.argv[2];fs.writeFileSync(p,JSON.stringify(j,null,2)+"\n");' "$file" "$version"
}

set_cargo_version "$root/server/Cargo.toml"
set_cargo_version "$root/desktop/src-tauri/Cargo.toml"
set_json_version  "$root/desktop/package.json"
set_json_version  "$root/desktop/src-tauri/tauri.conf.json"

echo "version set to $version in:"
echo "  server/Cargo.toml"
echo "  desktop/src-tauri/Cargo.toml"
echo "  desktop/package.json"
echo "  desktop/src-tauri/tauri.conf.json"
