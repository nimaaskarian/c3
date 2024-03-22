#!/usr/bin/env sh
. ./.shell-methods.sh

LAST_TAG=$(git tag | tail -n 1)

PACKAGE_NAME=c3
USERNAME=nimaaskarian
TAG="$1"

push_tag() {
  git tag "$TAG" || {
    echo List of existing tags:
    git tag
    echo_exit tag exists.
  }
  git push --tags
}

release_package() {
  cp target/release/c3 c3.x86.linux || echo_exit copy linux binary failed
  cp target/x86_64-pc-windows-gnu/release/c3.exe c3.x86_64.windows.exe || echo_exit copy windows binary failed
  FILES="c3.x86.linux c3.x86_64.windows.exe"
  gh release create "$TAG" $FILES --title "$TAG" --notes "**Full Changelog**: https://github.com/$USERNAME/$PACKAGE_NAME/compare/$LAST_TAG...$TAG" --repo $USERNAME/$PACKAGE_NAME
  rm $FILES
}

release_c3() {
  cd aur || echo_exit cd to aur directory failed.
  sed -i "s/pkgver=$LAST_TAG/pkgver=$TAG/" c3/PKGBUILD
  wget "https://github.com/$USERNAME/$PACKAGE_NAME/archive/refs/tags/$TAG.zip"
  MD5=$(md5sum $TAG.zip | cut -f 1 -d ' ')
  sed -i "s/md5sums=('.*')/md5sums=('$MD5')/" c3/PKGBUILD
  rm $TAG.zip

  cd c3 || echo_exit cd to c3 failed
  makepkg --printsrcinfo > .SRCINFO
  git add .
  git commit -m "Bumped version $TAG"
  git push
  cd ../..
}

release_c3_bin() {
  MD5=$(md5sum target/release/$PACKAGE_NAME | cut -f 1 -d ' ')
  cd aur || echo_exit cd to aur directory failed.
  sed -i "s/pkgver=$LAST_TAG/pkgver=$TAG/" c3-bin/PKGBUILD
  sed -i "s/md5sums=('.*')/md5sums=('$MD5')/" c3-bin/PKGBUILD

  cd c3-bin || echo_exit cd to c3 failed
  makepkg --printsrcinfo > .SRCINFO
  git add .
  git commit -m "Bumped version $TAG"
  git push
  cd ../..
}

push_tag
release_package
release_c3
release_c3_bin
