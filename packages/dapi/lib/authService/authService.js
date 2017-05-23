const Message = require('bitcore-message-dash');
class AuthService {
    constructor(app) {
        this.app = app;
        this.challengeMsg = Math.random().toString(36).substring(7); //pvr: pseudo random only, needs to be updated for production
    }
    //Triggered by graphQL ?
    resolveChallenge(txId, signature) {
        return this.app.rpc.getTransaction(txId)
            .then(txData => {
                    if(!txData || !txData.hasOwnProperty('vout'))
                    throw Error('Empty transaction');
                //pvr: move to bitcore-lib-dash?
                let rawData = txData.vout.filter(o => o.scriptPubKey.asm.includes('OP_RETURN'))[0]
                    .scriptPubKey.asm.replace('OP_RETURN ', '');
                let data = JSON.parse(new Buffer(rawData, 'hex').toString('utf8'));
                let pubKey = data.pubKey;
                ///////////////////////////////////
                return Message(this.challengeMsg).verify(pubKey, signature);
            }).catch(function(err){
                console.error('Error ', err)
            })
    }
}
module.exports = AuthService;