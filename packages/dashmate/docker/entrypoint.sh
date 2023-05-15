#!/bin/sh

echo "Running Dashmate Helper as: "
id

export DASHMATE_HOME_DIR=/home/dashmate/.dashmate
export DASHMATE_HELPER=1

exec $*
