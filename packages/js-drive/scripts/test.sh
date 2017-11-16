curl -i -X POST -H "Content-Type: application/json; indent=4" \
-d '{
"jsonrpc": "2.0",
"method": "getBlockchainUser",
"params": {"name": "andy"},
"id": "1"
}' http://localhost:5001;echo

curl -i -X POST -H "Content-Type: application/json; indent=4" \
-d '{
"jsonrpc": "2.0",
"method": "getBlockchainUserStateSinceHeight",
"params": {"name": "andy", "dapid": "1234"},
"id": "1"
}' http://localhost:5001;echo

curl -i -X POST -H "Content-Type: application/json; indent=4" \
-d '{
"jsonrpc": "2.0",
"method": "getBlockchainUserState",
"params": {"name": "andy", "dapid": "1234"},
"id": "1"
}' http://localhost:5001;echo

curl -i -X POST -H "Content-Type: application/json; indent=4" \
-d '{
"jsonrpc": "2.0",
"method": "getDapSchema",
"params": {"name": "andy", "dapid": "1234"},
"id": "1"
}' http://localhost:5001;echo
