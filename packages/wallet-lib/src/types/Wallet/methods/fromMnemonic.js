import {
  mnemonicToHDPrivateKey,
  is,
} from '../../../utils/index.js';
import DerivableKeyChain from '../../DerivableKeyChain/DerivableKeyChain.js';
import KeyChainStore from '../../KeyChainStore/KeyChainStore.js';
import { WALLET_TYPES } from '../../../CONSTANTS.js';

/**
 * Will set a wallet to work with a mnemonic (keychain, walletType & HDPrivateKey)
 * @param mnemonic
 */
export default function fromMnemonic(mnemonic, network, passphrase = '') {
  if (!is.mnemonic(mnemonic)) {
    throw new Error('Expected a valid mnemonic (typeof String or Mnemonic)');
  }
  const trimmedMnemonic = mnemonic.toString().trim();
  this.walletType = WALLET_TYPES.HDWALLET;
  // As we do not require the mnemonic except in this.exportWallet
  // users of wallet-lib are free to clear this prop at anytime.
  this.mnemonic = trimmedMnemonic;
  this.HDPrivateKey = mnemonicToHDPrivateKey(trimmedMnemonic, network, passphrase);

  this.keyChainStore = new KeyChainStore();
  const keyChain = new DerivableKeyChain({ HDPrivateKey: this.HDPrivateKey });
  this.keyChainStore.addKeyChain(keyChain, { isMasterKeyChain: true });
};
