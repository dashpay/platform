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

DIR="$( cd -P "$( dirname "$BASH_SOURCE[0]" )" >/dev/null 2>&1 && pwd )"

cd "${DIR}/.."

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

if [ -z "$DAPI_SEED" ] || [[ $DAPI_SEED == -* ]]
then
  echo "Seed is not specified"
  echo ""
  echo "$cmd_usage"
  exit 1
fi

if [ -n "$npm_package_to_install" ]
then
  npm install "$npm_package_to_install"
fi

if [ -n "$scope" ]
then
  scope_dirs=""

  IFS=', ' read -r -a scopes <<< "$scope"

  for scope in "${scopes[@]}"
    do
      case $scope in
        e2e)
          scope_dirs="${scope_dirs} test/e2e/**/*.spec.js"
        ;;
        functional)
          scope_dirs="${scope_dirs} test/functional/**/*.spec.js"
        ;;
        core)
          scope_dirs="${scope_dirs} test/functional/core/**/*.spec.js test/e2e/**/*.spec.js"
        ;;
        platform)
          scope_dirs="${scope_dirs} test/functional/platform/**/*.spec.js test/e2e/**/*.spec.js"
        ;;
        e2e:dpns)
          scope_dirs="${scope_dirs} test/e2e/dpns.spec.js"
        ;;
        e2e:contacts)
          scope_dirs="${scope_dirs} test/e2e/contacts.spec.js"
        ;;
        functional:core)
          scope_dirs="${scope_dirs} test/functional/core/**/*.spec.js"
        ;;
        functional:platform)
          scope_dirs="${scope_dirs} test/functional/platform/**/*.spec.js"
        ;;
      *)
      echo "Unknown scope $scope"
      exit 1
      ;;
    esac
  done
else
  scope_dirs="test/functional/**/*.spec.js test/e2e/**/*.spec.js"
fi

DAPI_SEED=${DAPI_SEED} FAUCET_PRIVATE_KEY=${faucet_key} NODE_ENV=test node_modules/.bin/mocha ${scope_dirs}
