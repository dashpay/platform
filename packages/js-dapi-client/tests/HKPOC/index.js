const client = require('../../HKPOC')


let req = {
    "jsonrpc": "2.0",
    "method": "getUser",
    "params": ["Alice"],
    "id": 3
}

client.getUser('alice')
    .then(res => {
        return res.result
    })
    .then(userRes => {
        return client.createUser(userRes.uname, userRes.pubkey, "INxlyf9JX8j1hfcBNf5W72+UALu7+5nF8l/MfUZSJlomNDkmwWfvva4eE4/tjJPUO+ByV6K3cbUgPhjbEZIM8Ik=")
    })
    .then(createRes => {
        console.log(createRes)
    })


