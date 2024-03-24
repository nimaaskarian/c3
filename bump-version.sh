#!/usr/bin/env sh
. ./.shell-methods.sh
TAG="$1"

bump_version() {
  sed -i "s/^version = .*/version = \"$TAG\"/" Cargo.toml
  git add Cargo.toml
}

build_package() {
  cargo build --release || echo_exit Linux build failed.
  cargo build --release --target x86_64-pc-windows-gnu || echo_exit Windows build failed.
}

cargo test || echo_exit Unittests failed.
bump_version
build_package
git add Cargo.lock
git commit -m "Bumped version $TAG"
