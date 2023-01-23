#! /bin/bash

set -e

YARN_CACHE=/cache/yarn
DOWNLOADS_CACHE=/cache/downloads
CPUS=4
DISK=50G
MEM=8G

function show_help {
    cat <<EOF
$0

This is a one-liner script to set up your test instance of Dash Platform inside a VM.
It will start a new Ubuntu 22.04 VM using multipass, and provision local Platform instance.

It uses:

- ubuntu 22.04 (recommended)
- multipass
- ${CPUS} CPU cores
- ${MEM} GB RAM
- ${DISK} GB HDD

Usage:

$0 [--branch BRANCHNAME] [--debug] [--mode MODE] [--name VMNAME] [--repo /path/to/repo/on/host]

where:

* --branch, -b - name of Platform branch to use; defaults to 'master'
* --cpus N - use N CPUs (defaults to ${CPUS})
* --cache DIR - use DIR to cache some files
* --debug, -d - more verbose output
* --mode - mode of operation:
   * multipass - start Dash Platform using multipass
   * cleanup - delete and purge multipass Dash Platform instance
* --name , -n - name of multipass instance to be created; defaults to 'dash'
* --repo - path on the host machine to the Platform repository to use; if not set, repository will be checked out inside the VM

Example:

./launch-platform.sh --branch v0.24-dev --cache ./cache/ --name dash1

EOF
}



function start_multipass {
    if ! which multipass >/dev/null; then
        echo 'Please install multipass; see https://multipass.run/ for more details.'
        exit 1
    fi

    multipass launch --cpus "${CPUS}" --disk "${DISK}" --memory "${MEM}" --name "${VMNAME}" 22.04

    if [[ -n "${CACHE}" ]]; then
        mkdir -p "${CACHE}"
        multipass mount "${CACHE}" "${VMNAME}:/cache"
    fi

    if [[ -n "${REPO}" ]]; then
        multipass mount "${REPO}" "${VMNAME}:/home/ubuntu/platform"
    fi

    multipass transfer "$0" "${VMNAME}:/home/ubuntu/setup-multipass.sh"
    multipass exec "${VMNAME}" -- /bin/bash /home/ubuntu/setup-multipass.sh --mode 'local' --name "${VMNAME}" --branch "${BRANCH}"
}


function setup_cache {
    sudo mkdir -p /cache/downloads /cache/yarn /cache/apt-archives
    sudo chown ${UID}:0 /cache/{downloads,yarn}
    chmod 770  /cache/{downloads,yarn}

    sudo chown root:root /cache/apt-archives
    sudo chmod 755 /cache/apt-archives
    sudo rm -r /var/cache/apt/archives
    sudo ln -s  /cache/apt-archives /var/cache/apt/archives || true
}


function cleanup_multipass {
    if ! which multipass >/dev/null; then
        echo 'Please install multipass; see https://multipass.run/ for more details.'
        exit 1
    fi

    multipass delete "${VMNAME}"
    multipass purge
}

function parse_args {
    while [ "$#" -ge 1 ]; do
        arg="$1"
        case $arg in
        -b | --branch)
            shift
            BRANCH="$1"
            shift
            ;;
        --cache)
            shift
            CACHE="$(realpath "$1")"
            shift
            ;;
        --cpus)
            shift
            CPUS="$1"
            shift
            ;;
        -d | --debug)
            DEBUG=1
            shift
            ;;
        -h | --help)
            MODE=help
            shift
            ;;
        --mode)
            shift
            MODE="$1"
            shift
            ;;
        -n | --name)
            shift
            VMNAME="$1"
            shift
            ;;
        --repo)
            shift
            REPO="$(realpath "$1")"
            shift
            ;;

        *)
            echo "Unrecoginzed command line argument '$arg';  try '$0 --help'"
            exit 1
            ;;
        esac
    done

    if [[ -n "$DEBUG" ]]; then
        set -x
    fi

    VMNAME="${VMNAME:-dash}"
    if [[ -n "${REPO}" ]] && [[ -n "${BRANCH}" ]] ; then
      echo 'ERROR: --repo and --branch are mutually exclusive, please use only one of them.'
      exit 1
    fi

    BRANCH="${BRANCH:-master}"
    MODE="${MODE:-multipass}"

}

# Download with caching
# Usage: download SRC_URL FILENAME
function download {
    FILENAME="$2"
    if [[ -z "${FILENAME}" ]] ; then
        FILENAME="$(basename "$1")"
    fi
    if [[ ! -f "${DOWNLOADS_CACHE}/${FILENAME}" ]] ; then
        wget --continue -O "${DOWNLOADS_CACHE}/${FILENAME}" "$1"
    fi
}



function install_deps {
    export DEBIAN_FRONTEND=noninteractive
    export TZ=UTC
    sudo apt-get update
    sudo apt-get install -y tzdata
    sudo ln -fs /usr/share/zoneinfo/UTC /etc/localtime
    sudo dpkg-reconfigure --frontend noninteractive tzdata
    sudo apt-get --yes install build-essential \
        pkg-config \
        wget \
        cmake \
        git \
        python3 \
        ca-certificates \
        curl \
        gnupg \
        lsb-release \
        libssl-dev \
        clang
}

function install_rust {
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain none -y
    bash -l -c "rustup install stable" # nightly
    bash -l -c "rustup target add wasm32-unknown-unknown"
#    sudo apt-get --yes install rustc cargo
#    sudo apt-get --yes install --no-install-recommends librust-cargo+openssl-dev
}

function install_docker {
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
    echo \
        "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu \
    $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list >/dev/null
    sudo apt-get update && sudo apt-get install --yes docker-ce docker-ce-cli containerd.io docker-compose-plugin
    sudo usermod -aG docker "$USER"
}

function install_nodejs {
    download https://nodejs.org/dist/v16.13.1/node-v16.13.1-linux-x64.tar.xz node.tar.xz
    tar xJf ${DOWNLOADS_CACHE}/node.tar.xz
    sudo mv node-v16.13.1-linux-x64 /opt/node
    sudo ln -s /opt/node/bin/* /usr/local/bin/
    sudo corepack enable
    yarn config set cache-folder "${YARN_CACHE}"
}

function install_platform {
    if [[ ! -d /home/ubuntu/platform ]]; then
        git clone --depth 1 --branch "${BRANCH}" https://github.com/dashevo/platform /home/ubuntu/platform
    fi
    cd /home/ubuntu/platform
    # sudo to ensure our group membership is updated to include `docker` group
    sudo -u "$USER" bash -l -c "yarn install"
    sudo -u "$USER" bash -l -c "yarn build"
}

function start_platform {
    sudo -u "$USER" bash -l -c "yarn setup"
    sudo -u "$USER" bash -l -c "yarn start"
}

function debug {
    [[ -n "$DEBUG" ]] && echo "$@"
}


### MAIN CODE ###

parse_args "$@"

case "$MODE" in
multipass)
    start_multipass
    echo "Platform started. Use multipass to access it, for example: multipass shell ${VMNAME}"
    ;;
cleanup)
    cleanup_multipass
    ;;
local)
    mkdir -p /usr/local/bin /opt

    setup_cache
    install_deps
    install_rust
    install_docker
    install_nodejs
    install_platform
    start_platform
    ;;
help)
    show_help
    ;;
*)
    echo "ERROR: Invalid mode $MODE; see $0 --help"
    ;;
esac
