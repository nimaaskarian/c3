#!/bin/sh
. ./.shell-methods.sh

repos="c3 c3-bin"
mkdir aur
cd aur || echo_exit cding to aur failed miserably.

for repo in $repos; do
  git clone ssh://aur@aur.archlinux.org/$repo.git
done
