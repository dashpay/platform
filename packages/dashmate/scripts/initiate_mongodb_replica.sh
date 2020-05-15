#!/bin/bash

echo "Starting replica set initialize"
until mongo --host drive_mongodb --eval "print(\"waited for connection\")"
do
    sleep 2
done
echo "Connection finished"
echo "Creating replica set"
mongo --host drive_mongodb <<EOF
rs.initiate({
      _id: "driveDocumentIndices",
      version: 1,
      members: [
         { _id: 0, host : "drive_mongodb:27017" },
      ]
   })
EOF
echo "Replica set created"
