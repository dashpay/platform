#!/usr/bin/env bash

set -ea

cmd_usage="Run test suite

Usage: test <seed> [options]

  <seed> can be IP or IP:port

  Options:
  -ni=pkg   --npm-install=pkg   - install npm package before running the suite
  -s=a,b,c  --scope=a,b,c       - test scope to run
  -k=key    --faucet-key=key    - faucet private key string
  -h        --help              - show help

  Possible scopes:
  e2e
  functional
  core
  platform
  e2e:dpns
  e2e:contacts
  functional:core
  functional:platform"

DAPI_SEED="$1"

if [ -z "$DAPI_SEED" ] || [[ $DAPI_SEED == -* ]]
then
  echo "Seed is not specified"
  exit 0
fi

for i in "$@"
do
case ${i} in
    -h|--help)
        echo "$cmd_usage"
        exit 0
    ;;
    -ni=*|--npm-install=*)
    npm_package_to_install="${i#*=}"
    ;;
    -s=*|--scope=*)
    scope="${i#*=}"
    ;;
    -k=*|--faucet-key=*)
    faucet_key="${i#*=}"
    ;;
esac
done

if [ -n "$npm_package_to_install" ]
then
  cd .. && npm install "$npm_package_to_install"
fi

if [ -n "$scope" ]
then
  cd .. && DAPI_SEED="$DAPI_SEED" FAUCET_PRIVATE_KEY="$faucet_key" npm run test:"$scope"
else
  cd .. && DAPI_SEED="$DAPI_SEED" FAUCET_PRIVATE_KEY="$faucet_key" npm run test
fi
