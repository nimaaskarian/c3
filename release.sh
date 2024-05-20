#!/usr/bin/env sh
. ./.shell-methods.sh

LAST_TAG=$(git tag | tail -n 2 | head -n 1)

PACKAGE_NAME=c3
USERNAME=nimaaskarian
TAG="$1"
SOURCE_MD5=""

make_tag() {
  git tag "$TAG" 2> /dev/null || {
    echo_warning tag "$TAG" already exists.
  }
}

release_package() {
  cp target/release/c3 c3.x86.linux || echo_exit copy linux binary failed
  cp target/x86_64-pc-windows-gnu/release/c3.exe c3.x86_64.windows.exe || echo_exit copy windows binary failed
  SOURCE=source.tar.gz
  git archive --output=$SOURCE --prefix=c3-$TAG/ $TAG -9 || echo_exit git archive $SOURCE failed
  SOURCE_MD5=$(md5sum $SOURCE | cut -f 1 -d ' ')
  FILES="c3.x86.linux c3.x86_64.windows.exe $SOURCE"
  gh release create "$TAG" $FILES --title "$TAG" --notes "**Full Changelog**: https://github.com/$USERNAME/$PACKAGE_NAME/compare/$LAST_TAG...$TAG" --repo $USERNAME/$PACKAGE_NAME
  rm $FILES
}

release() {
  WD=$PWD
  FOLDER=$1
  MD5=$2
  sed -i "s/pkgver=$LAST_TAG/pkgver=$TAG/" $FOLDER/PKGBUILD || echo_exit changing version of $FOLDER/PKGBUILD failed
  sed -i "s/pkgrel=.*/pkgrel=1/" $FOLDER/PKGBUILD || echo_exit setting rel of $FOLDER/PKGBUILD to 1 failed
  sed -i "s/md5sums=('.*')/md5sums=('$MD5')/" $FOLDER/PKGBUILD || echo_exit changing md5sum of $FOLDER/PKGBUILD failed

  cd $FOLDER || echo_exit cd to $FOLDER failed
  makepkg --printsrcinfo > .SRCINFO
  git add .
  git commit -m "Bumped version $TAG"
  git push
  cd "$WD" || echo_exit "cding back to previous working directory in release() failed"
}

make_tag
BIN_MD5=$(md5sum target/release/$PACKAGE_NAME | cut -f 1 -d ' ')
release_package
release ./aur/c3 "$SOURCE_MD5"
release ./aur/c3-bin "$BIN_MD5"
