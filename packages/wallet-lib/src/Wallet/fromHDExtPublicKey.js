const Dashcore = require('@dashevo/dashcore-lib');
const { is } = require('../utils/index');
const KeyChain = require('../KeyChain');
const { WALLET_TYPES } = require('../CONSTANTS');


const normalizeHDPubKey = key => (is.string(key) ? Dashcore.HDPublicKey(key) : key);
/**
 * Will set a wallet to work with a on readonly mode from a HDPublicKey
 * @param HDPublicKey
 */
module.exports = function fromHDPublicKey(_hdPublicKey) {
  if (!is.HDPublicKey(_hdPublicKey)) throw new Error('Expected a valid HDExtPublic key (typeof HDExtPublicKey or String)');
  this.walletType = WALLET_TYPES.HDEXTPUBLIC;
  this.mnemonic = null;
  const hdPublicKey = normalizeHDPubKey(_hdPublicKey);
  this.HDExtPublicKey = hdPublicKey;
  this.keyChain = new KeyChain({ HDPublicKey: hdPublicKey });
};
