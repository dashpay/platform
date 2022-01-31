const { DIP9_LIVENET_ROOT_PATH, DIP9_TESTNET_ROOT_PATH } = require('../../../CONSTANTS');

/**
 * Return a safier root path to derivate from
 * @return {HDPrivateKey|HDPublicKey}
 */
function getHardenedDIP9FeatureHDKey() {
  const pathRoot = (this.network.toString() === 'testnet') ? DIP9_TESTNET_ROOT_PATH : DIP9_LIVENET_ROOT_PATH;
  return this.getForPath(pathRoot).key;
}
module.exports = getHardenedDIP9FeatureHDKey;
