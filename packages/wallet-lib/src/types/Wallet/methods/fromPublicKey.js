import { is } from '../../../utils/index.js';
import DerivableKeyChain from '../../DerivableKeyChain/DerivableKeyChain.js';
import { WALLET_TYPES } from '../../../CONSTANTS.js';
import KeyChainStore from '../../KeyChainStore/KeyChainStore.js';

/**
 * Will set a wallet to work with a mnemonic (keychain, walletType & HDPrivateKey)
 * @param privateKey
 */
export default function fromPublicKey(publicKey, network) {
  if (!is.publicKey(publicKey)) throw new Error('Expected a valid public key (typeof PublicKey or String)');
  this.walletType = WALLET_TYPES.PUBLICKEY;
  this.mnemonic = null;
  this.publicKey = publicKey;

  const keyChain = new DerivableKeyChain({ publicKey, network });
  this.keyChainStore = new KeyChainStore();
  this.keyChainStore.addKeyChain(keyChain, { isMasterKeyChain: true });
};
