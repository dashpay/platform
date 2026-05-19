import { is } from '../../../utils/index.js';
import DerivableKeyChain from '../../DerivableKeyChain/DerivableKeyChain.js';
import { WALLET_TYPES } from '../../../CONSTANTS.js';
import KeyChainStore from '../../KeyChainStore/KeyChainStore.js';

/**
 * Will set a wallet to work with a mnemonic (keychain, walletType & HDPrivateKey)
 * @param privateKey
 */
export default function fromPrivateKey(privateKey, network) {
  if (!is.privateKey(privateKey)) throw new Error('Expected a valid private key (typeof PrivateKey or String)');
  this.walletType = WALLET_TYPES.PRIVATEKEY;
  this.mnemonic = null;
  this.privateKey = privateKey;

  const keyChain = new DerivableKeyChain({ privateKey, network });
  this.keyChainStore = new KeyChainStore();
  this.keyChainStore.addKeyChain(keyChain, { isMasterKeyChain: true });
};
