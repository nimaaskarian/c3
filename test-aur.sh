#!/usr/bin/env sh
. ./.shell-methods.sh

CHROOT=$HOME/.chroot
mkdir "$CHROOT"
mkarchroot "$CHROOT/root" base-devel

build_folder() {
  WD=$PWD
  cd $1 || echo_exit cd $1 failed
  makechrootpkg -c -r "$CHROOT" || echo_exit makechrootpkg $CHROOT/root failed
  cd "$WD"
}

build_folder ./aur/c3
build_folder ./aur/c3-bin
