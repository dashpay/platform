import { is } from '../../../utils/index.js';
import DerivableKeyChain from '../../DerivableKeyChain/DerivableKeyChain.js';
import { WALLET_TYPES } from '../../../CONSTANTS.js';
import KeyChainStore from '../../KeyChainStore/KeyChainStore.js';

/**
 * @param address
 */
export default function fromAddress(address, network) {
  if (!is.address(address)) throw new Error('Expected a valid address (typeof Address or String)');
  this.walletType = WALLET_TYPES.ADDRESS;
  this.mnemonic = null;
  this.address = address.toString();

  const keyChain = new DerivableKeyChain({ address, network });
  this.keyChainStore = new KeyChainStore();
  this.keyChainStore.addKeyChain(keyChain, { isMasterKeyChain: true });
};
