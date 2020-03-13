#!/bin/bash

echo "Starting replica set initialize"
until mongo --host drive_mongodb --eval "print(\"waited for connection\")"
do
    sleep 2
done
echo "Connection finished"
echo "Creating replica set"
mongo --host drive_mongodb <<EOF
rs.initiate()
EOF
echo "Replica set created"
