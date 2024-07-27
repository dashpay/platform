#!/usr/bin/env bash

set -ea

cmd_usage="Run test suite

Usage: test <seed> [options]

  <seed> can be IP or IP:port (or pass via DAPI_SEED env)

  Options:
  -s=a,b,c    --scope=a,b,c                                 - test scope to run
  -k=key      --faucet-key=key                              - faucet private key string
  -n=network  --network=network                             - use regtest, devnet or testnet
              --skip-sync-before-height=H                   - start sync funding wallet from specific height
              --faucet-wallet-use-storage=true              - use persistent wallet storage for faucet
              --faucet-wallet-storage-dir=absolute_dir      - specify directory where faucet wallet persistent storage will be stored
  -b          --bail                                        - bail after first test failure
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

DIR="$( cd -P "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

cd "${DIR}/.."

for i in "$@"
do
case ${i} in
    -h|--help)
        echo "$cmd_usage"
        exit 0
    ;;
    -s=*|--seed=*)
      seed="${i#*=}"
    ;;
    --scope=*)
    scope="${i#*=}"
    ;;
    -k=*|--faucet-key=*)
    faucet_key="${i#*=}"
    ;;
    -n=*|--network=*)
    network="${i#*=}"
    ;;
    --skip-sync-before-height=*)
    skip_sync_before_height="${i#*=}"
    ;;
    -b|--bail)
    bail=true
    ;;
    -t=*|--timeout=*)
    timeout="${i#*=}"
    ;;
    --faucet-wallet-storage-dir=*)
    faucet_wallet_storage_dir="${i#*=}"
    ;;
    --faucet-wallet-use-storage=*)
    faucet_wallet_use_storage="${i#*=}"
    ;;
esac
done

if [[ -n "$seed" ]]
then
  cmd="DAPI_SEED=${DAPI_SEED}"
fi

if [ -n "$timeout" ] && ! [[ $timeout =~ ^[0-9]+$ ]]
then
  echo "Timeout must be an integer"
  exit 1
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

if [ -n "$faucet_key" ]
then
  cmd="${cmd} FAUCET_PRIVATE_KEY=${faucet_key}"
fi

if [ -n "$network" ]
then
  cmd="${cmd} NETWORK=${network}"
fi

if [ -n "$skip_sync_before_height" ]
then
  cmd="${cmd} SKIP_SYNC_BEFORE_HEIGHT=${skip_sync_before_height}"
fi

if [ -n "$faucet_wallet_use_storage" ]
then
  cmd="${cmd} FAUCET_WALLET_USE_STORAGE=${faucet_wallet_use_storage}"
fi

if [ -n "$faucet_wallet_storage_dir" ]
then
  cmd="${cmd} FAUCET_WALLET_STORAGE_DIR=${faucet_wallet_storage_dir}"
fi

cmd="${cmd} NODE_ENV=test yarn mocha"

if [ -n "$bail" ]
then
  cmd="${cmd} -b"
fi

cmd="${cmd} ${scope_dirs}"

if [ -n "$timeout" ]
then
  cmd="${cmd} --timeout ${timeout}"
fi

eval $cmd
