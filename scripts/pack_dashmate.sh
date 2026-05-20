#!/bin/bash

set -e

cmd_usage="Usage: pack_dashmate.sh COMMAND

  COMMANDS:
    deb       pack into debian package
    macos     pack into macOS .pkg
    tarballs  packages into tarballs
    win       create windows installer
"
COMMAND="$1"

if [ -z "$COMMAND" ]
then
  echo "$cmd_usage"
  exit 1
fi

FLAGS=""

# Node 24+ does not publish 32-bit binaries (linux-armv7l, win-x86), so we
# must explicitly drop those targets from oclif's default lists; otherwise
# `oclif pack` 404s while downloading the embedded Node runtime.
#   - tarballs default: oclif builds whatever we pass; we previously passed
#     `linux-arm` (= armv7l). Replace with linux-arm64.
#   - deb default targets: linux-x64, linux-arm (armv7l), linux-arm64 — drop arm.
#   - win default targets: win32-x64, win32-x86 — drop x86.
case "$COMMAND" in
  tarballs)
    FLAGS="--no-xz --targets=linux-arm64,linux-x64"
    ;;
  deb)
    FLAGS="--targets=linux-x64,linux-arm64"
    ;;
  win)
    FLAGS="--targets=win32-x64"
    ;;
esac

FULL_PATH=$(realpath "$0")
DIR_PATH=$(dirname "$FULL_PATH")
ROOT_PATH=$(dirname "$DIR_PATH")

cd $ROOT_PATH/packages/dashmate || exit 1
yarn pack --install-if-needed
tar zxvf package.tgz -C .
cd $ROOT_PATH/packages/dashmate/package || exit 1
cp $ROOT_PATH/yarn.lock ./yarn.lock
mkdir .yarn
echo "nodeLinker: node-modules"  > .yarnrc.yml
yarn install --no-immutable
yarn oclif manifest
yarn oclif pack $COMMAND $FLAGS
cd ..  || exit 1
rm package.tgz
rm -rf package/dist/Release
rm -rf package/dist/Packages
cp -R package/dist "$ROOT_PATH/packages/dashmate"

# fix for deb package build
sudo chown -R $USER "$ROOT_PATH/packages/dashmate/package" || true
sudo chgrp -R $USER "$ROOT_PATH/packages/dashmate/package" || true

# remove build folder
rm -rf package

echo "Done"
