const { DIP9_LIVENET_ROOT_PATH, DIP9_TESTNET_ROOT_PATH } = require('../../../CONSTANTS');

/**
 * Return a safier root path to derivate from
 * @param type - {HDPrivateKey|HDPublicKey} def : HDPrivateKey - set the type of returned keys
 */
function getHardenedDIP9FeaturePath(type = 'HDPrivateKey') {
  const pathRoot = (this.network.toString() === 'testnet') ? DIP9_TESTNET_ROOT_PATH : DIP9_LIVENET_ROOT_PATH;
  return this.generateKeyForPath(pathRoot, type);
}
module.exports = getHardenedDIP9FeaturePath;
