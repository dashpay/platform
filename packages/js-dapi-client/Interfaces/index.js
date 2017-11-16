const jayson = require('jayson/promise');
const bitcored = require('bitcore-lib-dash')

var client = jayson.client.http('http://172.30.30.239:4019');

module.exports = {

    getUser: function(username) {
        return client.request('getUser', [username])
    },

    createUser: function(username, pubKey, privKey) {

        //todo: privKey.derive().getPrivKey().sign(xxx)
        let signature = "INxlyf9JX8j1hfcBNf5W72+UALu7+5nF8l/MfUZSJlomNDkmwWfvva4eE4/tjJPUO+ByV6K3cbUgPhjbEZIM8Ik="

        let preTx = {
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
        }

        let rawTx = null

        return client.request('createUser', rawTx)

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
