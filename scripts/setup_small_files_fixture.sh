#!/usr/bin/env bash
set -euo pipefail

target="${1:-target/small-files/vite}"
repo="${GPUHASH_SMALL_FILES_REPO:-https://github.com/vitejs/vite.git}"
commit="${GPUHASH_SMALL_FILES_COMMIT:-d64a1a5557b3caea9469e70b647ff2c9d9def809}"

if [[ ! -d "$target/.git" ]]; then
  rm -rf "$target"
  mkdir -p "$(dirname "$target")"
  git clone --filter=blob:none "$repo" "$target"
fi

git -C "$target" fetch --depth=1 origin "$commit"
git -C "$target" checkout --detach "$commit"

pushd "$target" >/dev/null
package_manager="$(node -p "require('./package.json').packageManager")"
corepack enable
corepack prepare "$package_manager" --activate
pnpm install --frozen-lockfile
popd >/dev/null

echo "$target"
