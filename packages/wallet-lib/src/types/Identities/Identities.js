const _ = require('lodash');
const Wallet = require('../Wallet/Wallet');

class Identities {
  constructor(wallet) {
    if (!wallet || wallet.constructor.name !== Wallet.name) throw new Error('Expected wallet to be passed as param');
    if (!_.has(wallet, 'walletId')) throw new Error('Missing walletID to create an account');

    this.walletId = wallet.walletId;

    this.storage = wallet.storage;

    this.keyChain = wallet.keyChain;
  }
}

Identities.prototype.getIdentityHDKeyById = require('./methods/getIdentityHDKeyById');
Identities.prototype.getIdentityHDKeyByIndex = require('./methods/getIdentityHDKeyByIndex');
Identities.prototype.getIdentityIds = require('./methods/getIdentityIds');

module.exports = Identities;
