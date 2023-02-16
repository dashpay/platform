#!/bin/bash

FULL_PATH=$(realpath "$0")
DIR_PATH=$(dirname "$FULL_PATH")
ROOT_PATH=$(dirname "$DIR_PATH")

cd $ROOT_PATH/packages/dashmate
yarn pack --install-if-needed
#tar zxvf package.tgz -C .
#cd package
#cp $ROOT_PATH/yarn.lock ./yarn.lock
#mkdir .yarn
#echo "nodeLinker: node-modules"  > .yarnrc.yml
#yarn install
#yarn oclif manifest
#yarn oclif pack macos
#cd ..
#rm package.tgz
#cp -R package/dist $ROOT_PATH
#rm -rf package
