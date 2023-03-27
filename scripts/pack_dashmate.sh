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
yarn oclif pack $COMMAND
cd ..  || exit 1
rm package.tgz
cp -R package/dist "$ROOT_PATH/packages/dashmate"
rm -rf package || true

echo "Done"
