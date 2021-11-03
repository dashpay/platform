#!/bin/bash

readarray -t dependants < <(jq -r '.[] | .[]' <<< "$1" )

pids=""
RESULT=0

for dependant in "${dependants[@]}"; do
  npm run test -w "$dependant" &
  pids="$pids $!"
done

for pid in $pids; do
  wait "$pid" || (( "RESULT=1" ))
done

if [ "$RESULT" == "1" ];
  then
    exit 1
fi
