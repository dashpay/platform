#!/usr/bin/env bash

## Setup arguments
while getopts t:a:l: flag
do
    case "${flag}" in
        t) target=${OPTARG};;
        a) arch=${OPTARG};;
        l) libc=${OPTARG};;
    esac
done

## Install multilib
apt update
apt install -y gcc-multilib
if [[ $target = "aarch64-unknown-linux-gnu" ]]
then
    apt install -y gcc-aarch64-linux-gnu libstdc++-11-dev-arm64-cross
fi

## Install Node.JS
curl -fsSL https://deb.nodesource.com/setup_16.x | sudo -E bash -
apt install -y nodejs

## Update toolchain
rustup update stable

## Install build target
rustup target install $target

chmod 777 -R /root/.cargo
mkdir -p /github/workspace/target
chmod 777 -R /github/workspace/target

ARCH=$arch LIBC=$libc npm run build -- --release --target=$target
