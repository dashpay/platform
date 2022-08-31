#!/usr/bin/env bash

set -e

PATH_TO_SCRIPT=$(realpath $0)
PATH_TO_SCRIPTS_DIRECTORY=$(dirname $PATH_TO_SCRIPT)
PATH_TO_PROJECT_ROOT=$(dirname $PATH_TO_SCRIPTS_DIRECTORY)
# 1 level above monorepo root
PATH_TO_ROOT=$(dirname $PATH_TO_PROJECT_ROOT)

DASH_NETWORK_CI_PATH=$(realpath "$PATH_TO_ROOT/dash-network-ci")
DASH_NETWORK_CONFIGS_PATH=$(realpath "$PATH_TO_ROOT/dash-network-configs")

NETWORK_NAME="$1"

# we must pass it in the script with argument
if [[ -z "$NETWORK_NAME" ]]; then
    echo "Network name must be specified"
    exit 1
fi

# if there is no dash-network configs folder in the upper dir
if  [ ! -f "$DASH_NETWORK_CONFIGS_PATH/$NETWORK_NAME.yml" ]; then
    echo "Dash network config '$NETWORK_NAME' was not found in the folder ($DASH_NETWORK_CONFIGS_PATH)"
    exit 1
fi

# if such network name could not be found in network dir
if [ ! -f "$DASH_NETWORK_CONFIGS_PATH/$NETWORK_NAME.yml" ]; then
    echo "Dash network config '$NETWORK_NAME' was not found in the folder ($DASH_NETWORK_CONFIGS_PATH)"
    exit 1
fi

cp $DASH_NETWORK_CONFIGS_PATH/$NETWORK_NAME* $DASH_NETWORK_CI_PATH/dash-network-configs/

$DASH_NETWORK_CI_PATH/scripts/configure_test_suite.sh $NETWORK_NAME
cp $DASH_NETWORK_CI_PATH/.env $PATH_TO_PROJECT_ROOT/packages/platform-test-suite

echo "Successfully generated .env for $NETWORK_NAME and copied in platform-test-suite"

#
#if [[ -f ".env" ]]; then
#    source ".env"
#fi
#
#if [[ -f "packages/platform-test-suite/.env" ]]; then
#    source "packages/platform-test-suite/.env"
#fi
