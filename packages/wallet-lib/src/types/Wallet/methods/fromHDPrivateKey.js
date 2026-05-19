import dashcore from '@dashevo/dashcore-lib';
const { HDPrivateKey } = dashcore;
import {
  is,
} from '../../../utils/index.js';
import DerivableKeyChain from '../../DerivableKeyChain/DerivableKeyChain.js';
import KeyChainStore from '../../KeyChainStore/KeyChainStore.js';
import { WALLET_TYPES } from '../../../CONSTANTS.js';

/**
 * Will set a wallet to work with a seed (HDPrivateKey)
 * @param hdPrivateKey
 */
export default function fromHDPrivateKey(hdPrivateKey) {
  if (!is.HDPrivateKey(hdPrivateKey)) throw new Error('Expected a valid HDPrivateKey (typeof HDPrivateKey or String)');
  this.walletType = WALLET_TYPES.HDWALLET;
  this.mnemonic = null;
  this.HDPrivateKey = HDPrivateKey(hdPrivateKey);

  const keyChain = new DerivableKeyChain({ HDPrivateKey: this.HDPrivateKey });
  this.keyChainStore = new KeyChainStore();
  this.keyChainStore.addKeyChain(keyChain, { isMasterKeyChain: true });
};
