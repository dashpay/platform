#!/usr/bin/env bash

set -ea

cmd_usage="Run test suite

Usage: test <seed> [options]

  <seed> can be IP or IP:port

  Options:
              --npm-install=pkg                             - install npm package before running the suite
  -s=a,b,c    --scope=a,b,c                                 - test scope to run
  -k=key      --faucet-key=key                              - faucet private key string
  -n=network  --network=network                             - use regtest or testnet
              --dpns-tld-identity-private-key=private_key   - top level identity private key
              --dpns-tld-identity-id=identity_id            - top level identity id
              --dpns-contract-id=contract_id                - dpns contract id
  -t          --timeout                                     - test timeout in milliseconds
  -h          --help                                        - show help

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
network="testnet"

DIR="$( cd -P "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

cd "${DIR}/.."

for i in "$@"
do
case ${i} in
    -h|--help)
        echo "$cmd_usage"
        exit 0
    ;;
    --npm-install=*)
    npm_package_to_install="${i#*=}"
    ;;
    -s=*|--scope=*)
    scope="${i#*=}"
    ;;
    -k=*|--faucet-key=*)
    faucet_key="${i#*=}"
    ;;
    -n=*|--network=*)
    network="${i#*=}"
    ;;
    --dpns-tld-identity-private-key=*)
    identity_private_key="${i#*=}"
    ;;
    --dpns-tld-identity-id=*)
    identity_id="${i#*=}"
    ;;
    --dpns-contract-id=*)
    contract_id="${i#*=}"
    ;;
    -t=*|--timeout=*)
    timeout="${i#*=}"
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

if [ -n "$timeout" ] && ! [[ $timeout =~ ^[0-9]+$ ]]
then
  echo "Timeout must be an integer"
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

cmd="DAPI_SEED=${DAPI_SEED}"

if [ -n "$faucet_key" ]
then
  cmd="${cmd} FAUCET_PRIVATE_KEY=${faucet_key}"
fi

if [ -n "$network" ]
then
  cmd="${cmd} NETWORK=${network}"
fi

if [ -n "$contract_id" ]
then
  cmd="${cmd} DPNS_CONTRACT_ID=${contract_id}"
fi

if [ -n "$identity_id" ]
then
  cmd="${cmd} DPNS_TOP_LEVEL_IDENTITY_ID=${identity_id}"
fi

if [ -n "$identity_private_key" ]
then
  cmd="${cmd} DPNS_TOP_LEVEL_IDENTITY_PRIVATE_KEY=${identity_private_key}"
fi

cmd="${cmd} NODE_ENV=test mocha ${scope_dirs}"

if [ -n "$timeout" ]
then
  cmd="${cmd} --timeout ${timeout}"
fi

eval $cmd
