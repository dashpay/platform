var jayson = require('jayson/promise');

var client = jayson.client.http('http://172.30.30.239:4019');

module.exports = {

    getUser: function(username) {
        return client.request('getUser', [username])
    },

    createUser: function(username, pubKey, signature) {
        return client.request('createUser', {
            "description": "valid - registration subtx meta for alice",
            "data": {
                "pver": 1,
                "objtype": "SubTx",
                "action": 1,
                "uname": `${username}`,
                "pubkey": `${pubKey}`
            },
            "meta": {
                "id": "ef6ab42e001144bfbaf4777b05148f56a9705b63cdc320c95171bc600df7088e",
                "sig": `${signature}`
            }
        })

    },

    getUserState: function(username, dapId) {
        return client.request('getUserState', {
            uname: `${username}`,
            dapId: `${dapId}`
        })
    },

    getUserState: function(username, dapId, height) {
        return client.request('getUserState', {
            uname: `${username}`,
            dapId: `${dapId}`,
            height: `${height}`
        })
    }
}
