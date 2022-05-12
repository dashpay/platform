#! /bin/bash

set -e

mkdir -p /usr/local/bin /opt

function show_help {
    cat <<EOF
$0

This is a one-liner script to set up your test instance of Dash Platform inside a VM.
It will start a new Ubuntu 22.04 VM using multipass, and provision local Platform instance.

You will need:

- ubuntu 22.04 (recommended)
- multipass
- 4 GB RAM
- 20 GB HDD

Usage:

$0 [--branch BRANCHNAME] [--debug] [--mode MODE] [--name VMNAME] [--repo /path/to/repo/on/host]

where:

* --branch, -b - name of Platform branch to use; defaults to 'master'
* --debug, -d - more verbose output
* --mode - mode of operation:
   * multipass - start Dash Platform using multipass
   * cleanup - delete and purge multipass Dash Platform instance
* --name , -n - name of multipass instance to be created; defaults to 'dash'
* --repo - path on the host machine to the Platform repository to use; if not set, repository will be checked out inside the VM


EOF
}

function start_multipass {
    if ! which multipass >/dev/null; then
        echo 'Please install multipass; see https://multipass.run/ for more details.'
        exit 1
    fi

    multipass launch --cpus 2 --disk 20G --mem 4G --name "${VMNAME}" 22.04

    if [[ -n "${REPO}" ]]; then
        multipass mount "${REPO}" "${VMNAME}:/home/ubuntu/platform"
    fi

    multipass transfer "$0" "${VMNAME}:/home/ubuntu/setup-multipass.sh"
    multipass exec "${VMNAME}" -- /bin/bash /home/ubuntu/setup-multipass.sh --mode 'local' --name "${VMNAME}"
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
    BRANCH="${BRANCH:-master}"
    MODE="${MODE:-multipass}"

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
        lsb-release
}

function install_docker {
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
    echo \
        "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu \
    $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list >/dev/null
    sudo apt-get update && sudo apt-get install --yes docker-ce docker-ce-cli containerd.io
    sudo usermod -aG docker "$USER"
}

function install_compose {
    # Docker Compose
    wget -O docker-compose https://github.com/docker/compose/releases/download/1.29.2/docker-compose-Linux-x86_64
    sudo mv docker-compose /usr/local/bin/docker-compose
    sudo chmod +x /usr/local/bin/docker-compose
}

function install_nodejs {
    wget https://nodejs.org/dist/v16.13.1/node-v16.13.1-linux-x64.tar.xz
    tar xJf node-v16.13.1-linux-x64.tar.xz
    rm -f node-v16.13.1-linux-x64.tar.xz
    sudo mv node-v16.13.1-linux-x64 /opt/node
    sudo ln -s /opt/node/bin/* /usr/local/bin/
    sudo corepack enable
}

function install_platform {
    if [[ ! -d /home/ubuntu/platform ]]; then
        git clone --depth 1 --branch "${BRANCH}" https://github.com/dashevo/platform /home/ubuntu/platform
    fi
    cd /home/ubuntu/platform
    # sudo to ensure our group membership is updated to include `docker` group
    sudo -u "$USER" yarn install
    sudo -u "$USER" yarn build
}

function start_platform {
    sudo -u "$USER" yarn setup
    sudo -u "$USER" yarn start
}

function debug {
    [[ -n "$DEBUG" ]] && echo "$@"
}

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
    install_deps
    install_docker
    install_compose
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
