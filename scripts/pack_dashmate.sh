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
# must drop those targets from oclif's defaults; otherwise `oclif pack` 404s
# while downloading the embedded Node runtime.
#   - `tarballs` and `win` accept `--targets`, so we override on the command line.
#   - `oclif pack deb` has no `--targets` flag; its linux targets come from
#     `oclif.update.node.targets` in packages/dashmate/package.json.
case "$COMMAND" in
  tarballs)
    FLAGS="--no-xz --targets=linux-arm64,linux-x64"
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
