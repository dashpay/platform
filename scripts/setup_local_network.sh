CONFIG_NAME="local"
MINING_INTERVAL_IN_SECONDS=30
MASTERNODES_COUNT=3

FULL_PATH=$(realpath $0)
DIR_PATH=$(dirname $FULL_PATH)
ROOT_PATH=$(dirname $DIR_PATH)
PACKAGES_PATH="$ROOT_PATH/packages"

"${PACKAGES_PATH}"/dashmate/bin/dashmate setup ${CONFIG_NAME} --verbose --debug-logs --miner-interval="${MINING_INTERVAL_IN_SECONDS}s" --node-count=${MASTERNODES_COUNT} | tee "${PACKAGES_PATH}"/dashmate/setup.log
