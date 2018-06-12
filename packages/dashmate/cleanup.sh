#!/usr/bin/env bash

NETWORK=$1
if [[ -z "$NETWORK" || ! -f networks/$NETWORK.env ]]; then
  echo "Please specify network name as first argument"
  exit 1
fi
shift 1

./mn-bootstrap.sh $NETWORK down &&
./mn-bootstrap.sh $NETWORK rm -fv &&

sudo su <<HERE
rm -rf ./data/core-regtest
HERE
