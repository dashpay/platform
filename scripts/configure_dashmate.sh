CONFIG_NAME="local"

full_path=$(realpath "$0")
dir_path=$(dirname "$full_path")
root_path=$(dirname "$dir_path")
packages_path="$root_path/packages"

DAPI_REPO_PATH="${packages_path}/dapi"
DRIVE_REPO_PATH="${packages_path}/js-drive"

"${packages_path}"/dashmate/bin/dashmate update -v
"${packages_path}"/dashmate/bin/dashmate config:set --config=${CONFIG_NAME} platform.dapi.api.docker.build.path "$DAPI_REPO_PATH"
"${packages_path}"/dashmate/bin/dashmate config:set --config=${CONFIG_NAME} platform.drive.abci.docker.build.path "$DRIVE_REPO_PATH"
