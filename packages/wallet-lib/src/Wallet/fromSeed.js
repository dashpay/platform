const {
  is,
} = require('../utils/index');
const KeyChain = require('../KeyChain');
const { WALLET_TYPES } = require('../CONSTANTS');

/**
 * Will set a wallet to work with a seed (HDPrivateKey)
 * fixme: Term seed is often use, but we might want to rename to fromHDPrivateKey
 * @param seed
 */
module.exports = function fromSeed(seed) {
  if (!is.seed(seed)) throw new Error('Expected a valid seed (typeof HDPrivateKey or String)');
  this.type = WALLET_TYPES.HDWALLET;
  this.mnemonic = null;
  this.HDPrivateKey = seed;
  this.keyChain = new KeyChain({ HDRootKey: seed });
};
