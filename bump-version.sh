#!/usr/bin/env sh
. ./.error-on-no-arg.sh
. ./.shell-methods.sh
TAG="$1"

bump_version() {
  sed -i "s/^version = .*/version = \"$TAG\"/" Cargo.toml
  git add Cargo.toml
}

cargo test || echo_exit Unittests failed.
bump_version
. $(dirname $0)/build.sh
git add Cargo.lock
git commit -m "Bumped version $TAG"
