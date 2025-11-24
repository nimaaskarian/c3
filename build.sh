#!/usr/bin/env sh
. ./.shell-methods.sh
cargo build --release || echo_exit Linux build failed.
cargo build --release --target x86_64-pc-windows-gnu || echo_exit Windows build failed.
export ANDROID_NDK_HOME=/opt/android-sdk/ndk/27.0.12077973
cargo ndk -t aarch64-linux-android build --release || echo_exit Termux build failed.
