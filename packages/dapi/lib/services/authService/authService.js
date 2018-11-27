// TODO: Address ESLint issues the next time this file is edited
/* eslint-disable */
const { Message } = require('@dashevo/dashcore-lib');
const inMemDb = require('./inMemDb');
const mocks = require('../../mocks/mocks');

class AuthService {
  constructor(app) {
    this.app = app;

    // pvr: To follow a stateless pattern I don't think
    // any additional properties should be created here
  }

  // Possibly depricated using 'auth on each request' model
  isValidTxId(txId) {
    return true;
    // pvr: this needs to be implented by using some checksum (probably in @dashevo/dashcore-lib)
  }

  // Possibly depricated using 'auth on each request' model
  getChallenge(identifier) {
    // pvr: pseudo random only, needs to be updated for production
    // In the case of multisig this will also be updated to send locking script instead of a str mesagge
    const challenge = Math.random().toString(36).substring(7);

    // pvr: For now local memory is used to keep track of state
    // will be reconcidered after 'stateless decentralized session management' problem has been solved
    inMemDb.setChallenge(identifier, challenge);

    return challenge;
  }

  getUserObj() {
    return new Promise(((resolve, reject) => {
      resolve(mocks.mocksUser);
    }));
  }

  resolveChallenge(username, nonce, signature) {
    return this.getUserObj(username)
      .then((userObj) => {
        if (nonce != userObj.Header.ObjNonce + 1) {
          return false;
        }
        return this.app.rpc.getTransaction(userObj.Header.RegTX);
      })
      .then((txData) => {
        // pvr: move to @dashevo/dashcore-lib?
        const rawData = txData.vout.filter(o => o.scriptPubKey.asm.includes('OP_RETURN'))[0]
          .scriptPubKey.asm.replace('OP_RETURN ', '');
        const data = JSON.parse(new Buffer(rawData, 'hex').toString('utf8'));
        const pubKey = data.pubKey;
        // /////////////////////////////////

        return Message(nonce).verify(pubKey, signature);
      }).catch((err) => {
        console.error('Error ', err);
        return false;
      });
  }
}
module.exports = AuthService;
