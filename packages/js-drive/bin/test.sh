#!/usr/bin/env bash

curl -i -X POST -H "Content-Type: application/json; indent=4" \
-d '{
"jsonrpc": "2.0",
"method": "addTransitionPackets",
"id": "1"
}' http://localhost:5001;echo

curl -i -X POST -H "Content-Type: application/json; indent=4" \
-d '{
"jsonrpc": "2.0",
"method": "getDapSchemaByVersion",
"id": "1"
}' http://localhost:5001;echo

curl -i -X POST -H "Content-Type: application/json; indent=4" \
-d '{
"jsonrpc": "2.0",
"method": "getDapSchemas",
"id": "1"
}' http://localhost:5001;echo

curl -i -X POST -H "Content-Type: application/json; indent=4" \
-d '{
"jsonrpc": "2.0",
"method": "getDapSchemaVersions",
"id": "1"
}' http://localhost:5001;echo

curl -i -X POST -H "Content-Type: application/json; indent=4" \
-d '{
"jsonrpc": "2.0",
"method": "getTransitionPacket",
"id": "1"
}' http://localhost:5001;echo

curl -i -X POST -H "Content-Type: application/json; indent=4" \
-d '{
"jsonrpc": "2.0",
"method": "getUserObjectByRevision",
"id": "1"
}' http://localhost:5001;echo

curl -i -X POST -H "Content-Type: application/json; indent=4" \
-d '{
"jsonrpc": "2.0",
"method": "getUserObjectRevisions",
"id": "1"
}' http://localhost:5001;echo

curl -i -X POST -H "Content-Type: application/json; indent=4" \
-d '{
"jsonrpc": "2.0",
"method": "getUserObjects",
"id": "1"
}' http://localhost:5001;echo
