const { BIP44_TESTNET_ROOT_PATH, BIP44_LIVENET_ROOT_PATH } = require('../../../CONSTANTS');

/**
 * Return a safier root path to derivate from
 * @param {HDPrivateKey|HDPublicKey} [type=HDPrivateKey] - set the type of returned keys
 * @return {HDPrivateKey|HDPublicKey}
 */
function getHardenedBIP44Path(type = 'HDPrivateKey') {
  const pathRoot = (this.network.toString() === 'testnet') ? BIP44_TESTNET_ROOT_PATH : BIP44_LIVENET_ROOT_PATH;
  return this.generateKeyForPath(pathRoot, type);
}
module.exports = getHardenedBIP44Path;
