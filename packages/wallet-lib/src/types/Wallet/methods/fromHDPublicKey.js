const Dashcore = require('@dashevo/dashcore-lib');
const { is } = require('../../../utils');
const KeyChain = require('../../KeyChain/KeyChain');
const { WALLET_TYPES } = require('../../../CONSTANTS');


const normalizeHDPubKey = (key) => (is.string(key) ? Dashcore.HDPublicKey(key) : key);
/**
 * Will set a wallet to work with a on readonly mode from a HDPublicKey
 * @param HDPublicKey
 */
module.exports = function fromHDPublicKey(_hdPublicKey) {
  if (!is.HDPublicKey(_hdPublicKey)) throw new Error('Expected a valid HDPublicKey (typeof HDPublicKey or String)');
  this.walletType = WALLET_TYPES.HDPUBLIC;
  this.mnemonic = null;
  this.HDPublicKey = normalizeHDPubKey(_hdPublicKey);
  this.keyChain = new KeyChain({ HDPublicKey: this.HDPublicKey });
};
