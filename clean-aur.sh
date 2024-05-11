#!/bin/sh

clean() {
  rm $1/*.tar.gz
  rm $1/*.log
  rm $1/*.zst
  rm -r $1/src
  rm -r $1/pkg
}
clean ./aur/c3
clean ./aur/c3-bin
