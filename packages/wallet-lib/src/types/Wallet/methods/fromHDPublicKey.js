import Dashcore from '@dashevo/dashcore-lib';
import { is } from '../../../utils/index.js';
import DerivableKeyChain from '../../DerivableKeyChain/DerivableKeyChain.js';
import { WALLET_TYPES } from '../../../CONSTANTS.js';
import KeyChainStore from '../../KeyChainStore/KeyChainStore.js';

const normalizeHDPubKey = (key) => (is.string(key) ? Dashcore.HDPublicKey(key) : key);
/**
 * Will set a wallet to work with a on readonly mode from a HDPublicKey
 * @param HDPublicKey
 */
export default function fromHDPublicKey(_hdPublicKey) {
  if (!is.HDPublicKey(_hdPublicKey)) throw new Error('Expected a valid HDPublicKey (typeof HDPublicKey or String)');
  this.walletType = WALLET_TYPES.HDPUBLIC;
  this.mnemonic = null;
  this.HDPublicKey = normalizeHDPubKey(_hdPublicKey);

  const keyChain = new DerivableKeyChain({ HDPublicKey: this.HDPublicKey });
  this.keyChainStore = new KeyChainStore();
  this.keyChainStore.addKeyChain(keyChain, { isMasterKeyChain: true });
};
