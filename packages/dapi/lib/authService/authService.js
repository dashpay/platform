'use strict'
const Message = require('bitcore-message-dash');
const inMemDb = require('./inMemDb');

class AuthService {

    constructor(app) {
        this.app = app;

        //pvr: To follow a stateless pattern I don't think 
        //any additional properties should be created here
    }

    isValidTxId(txId) {
        return true;
        //pvr: this needs to be implented by using some checksum (probably in bitcore-lib-dash)
    }

    getChallenge(txId) {

        //pvr: pseudo random only, needs to be updated for production
        //In the case of multisig this will also be updated to send locking script instead of a str mesagge
        var challenge = Math.random().toString(36).substring(7);

        //pvr: For now local memory is used to keep track of state
        //will be reconcidered after 'stateless decentralized session management' problem has been solved
        inMemDb.setChallenge(txId, challenge);

        return challenge;
    }

    resolveChallenge(txId, signature) {

        if (!this.isValidTxId(txId)) {
            console.log('invalid transaction id');
            return false;
        }
        else {
            this.app.rpc.getTransaction(txId)
                .then(txData => {
                    //pvr: move to bitcore-lib-dash?
                    let rawData = txData.vout.filter(o => o.scriptPubKey.asm.includes('OP_RETURN'))[0]
                        .scriptPubKey.asm.replace('OP_RETURN ', '');
                    let data = JSON.parse(new Buffer(rawData, 'hex').toString('utf8'));
                    let pubKey = data.pubKey;
                    ///////////////////////////////////

                    return Message(inMemDb.getChallenge(txId)).verify(pubKey, signature);

                }).catch(function(err) {
                    console.error('Error ', err);
                    return false;
                })
        }
    }

}
module.exports = AuthService;