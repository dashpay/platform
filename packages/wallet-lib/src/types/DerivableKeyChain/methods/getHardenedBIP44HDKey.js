import { BIP44_TESTNET_ROOT_PATH, BIP44_LIVENET_ROOT_PATH } from '../../../CONSTANTS.js';

/**
 * Return a safier root keys to derivate from
 * @return {HDPrivateKey|HDPublicKey}
 */
function getHardenedBIP44HDKey() {
  const pathRoot = (this.network.toString() === 'testnet') ? BIP44_TESTNET_ROOT_PATH : BIP44_LIVENET_ROOT_PATH;
  return this.getForPath(pathRoot).key;
}
export default getHardenedBIP44HDKey;