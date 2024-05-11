#!/usr/bin/env sh

RED='\033[0;31m'
NC='\033[0m' # No Color

echo_exit() {
 echo -e "${RED}Error$NC: $*"
 exit 1 
}

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
