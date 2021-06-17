#!/usr/bin/env bash

set -e

#COLLATERAL_KEY=
#COLLATERAL_ADDRESS=

# PLEASE PUT YOUR FAUCET KEY HERE
FAUCET_PRIVATE_KEY=
FAUCET_ADDRESS=
MINING_INTERVAL_IN_SECONDS=20

# PLEASE SET THIS VARIABLES TO YOUR LOCAL DIRECTORIES WITH THE CODE IF YOU WISH TO COMPILE DAPI AND DRIVE
DAPI_REPO_PATH=/dash/dapi
DRIVE_REPO_PATH=/dash/drive

BUILD_DAPI_BEFORE_SETUP=false
BUILD_DAPI_AFTER_SETUP=false
BUILD_DRIVE=false

CONFIG_NAME="local"

MASTERNODES_COUNT=3

echo "Removing all docker containers and volumes..."
docker rm -f -v $(docker ps -a -q) || true
docker system prune -f; rm -rf ~/.dashmate/

if [ $BUILD_DRIVE == true ]
then
  echo "Setting drive build directory"
  ./bin/dashmate config:set --config=${CONFIG_NAME} platform.drive.abci.docker.build.path $DRIVE_REPO_PATH
fi

if [ $BUILD_DAPI_BEFORE_SETUP == true ]
then
  echo "Setting dapi build directory before the setup"
  ./bin/dashmate config:set --config=${CONFIG_NAME} platform.dapi.api.docker.build.path $DAPI_REPO_PATH
fi

./bin/dashmate setup ${CONFIG_NAME} --verbose --debug-logs --miner-interval="${MINING_INTERVAL_IN_SECONDS}s" --node-count=${MASTERNODES_COUNT}

echo "Sending 1000 tDash to the ${FAUCET_ADDRESS} for tests"
./bin/dashmate wallet:mint 1000 --config=${CONFIG_NAME}_seed --address=${FAUCET_ADDRESS}

if [ $BUILD_DAPI_AFTER_SETUP == true ]
then
  echo "Setting dapi build directory after the setup"
  for (( NODE_INDEX=1; NODE_INDEX<=MASTERNODES_COUNT; NODE_INDEX++ ))
  do
    ./bin/dashmate config:set --config=${CONFIG_NAME}_${NODE_INDEX} platform.dapi.api.docker.build.path $DAPI_REPO_PATH
  done
fi

./bin/dashmate group:start --wait-for-readiness

echo "Funding key is ${FAUCET_PRIVATE_KEY}"
