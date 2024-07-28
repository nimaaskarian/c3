#!/usr/bin/env sh
. ./.error-on-no-arg.sh
. ./.shell-methods.sh
TAG="$1"

bump_version() {
  sed -i "s/^version = .*/version = \"$TAG\"/" Cargo.toml
  git add Cargo.toml
}

build_package() {
  cargo build --release || echo_exit Linux build failed.
  cargo build --release --target x86_64-pc-windows-gnu || echo_exit Windows build failed.
  export ANDROID_NDK_HOME=/opt/android-sdk/ndk/27.0.12077973
  cargo ndk -t aarch64-linux-android build --release || echo_exit Termux build failed.
}

cargo test || echo_exit Unittests failed.
bump_version
build_package
git add Cargo.lock
git commit -m "Bumped version $TAG"
