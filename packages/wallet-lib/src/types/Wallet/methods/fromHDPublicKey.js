const Dashcore = require('@dashevo/dashcore-lib');
const { is } = require('../../../utils');
const DerivableKeyChain = require('../../DerivableKeyChain/DerivableKeyChain');
const { WALLET_TYPES } = require('../../../CONSTANTS');
const KeyChainStore = require('../../KeyChainStore/KeyChainStore');

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

  const keyChain = new DerivableKeyChain({ HDPublicKey: this.HDPublicKey });
  this.keyChainStore = new KeyChainStore();
  this.keyChainStore.addKeyChain(keyChain, { isMasterKeyChain: true });
};
