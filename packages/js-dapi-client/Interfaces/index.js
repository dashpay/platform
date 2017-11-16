const jayson = require('jayson/promise');
const Transaction = require('bitcore-lib-dash').Transaction
const Bitcore = require('bitcore-lib-dash')

var client = jayson.client.http('http://172.30.30.239:4019');

module.exports = {

    getUser: function(username) {
        return client.request('getUser', [username])
    },

    createUser: function(username, pubKey, privateKey) {

        let accData = {
            "description": "valid - registration subtx meta for alice",
            "data": {
                "pver": 1,
                "objtype": "SubTx",
                "action": 1,
                "uname": `${username}`,
                "pubkey": `${pubKey}`
            }
        }

        var utxo = new Transaction.UnspentOutput({
            "txid": "a0a08e397203df68392ee95b3f08b0b3b3e2401410a38d46ae0874f74846f2e9",
            "vout": 0,
            "address": "ySw349uTAxzmckt2Z84EzMr2ApmuXBq472",
            "scriptPubKey": "76a914089acaba6af8b2b4fb4bed3b747ab1e4e60b496588ac",
            "amount": 0.00070000
        });

        let tx = new Transaction()
            .from(utxo)
            .addData(JSON.stringify(accData))
            .fee(100)
            .sign(privateKey)
            .serialize(true);

        return client.request('createUser', { rawTx: tx })

    },

    doFriendRequest: function() {

    },

    payContact: function(privateKey, toUsername, xPubkey, dapId) {
        client.request('getRelation', {
            uname: toUsername
        }).then(relId => {
            client.request('payContact', {
                pver: 1,
                objtype: 'DashPayContact',
                dapid: dapId || 0,
                actn: 0, //Add
                revn: 0, //?
                leafn: 1, //?
                relation: relId,
                hdextpubkey: new Bitcore.HDPrivateKey(privateKey).derive("m/1").xPubkey.toString(),
                agreetscs: 1  // accept the terms and conditions
            })
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
