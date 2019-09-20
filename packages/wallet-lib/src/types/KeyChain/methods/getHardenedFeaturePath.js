const { BIP44_TESTNET_ROOT_PATH, BIP44_LIVENET_ROOT_PATH } = require('../../../CONSTANTS');

/**
 * Return a safier root path to derivate from
 *
 */
function getHardenedFeaturePath() {
  const pathRoot = (this.network.toString() === 'testnet') ? BIP44_TESTNET_ROOT_PATH : BIP44_LIVENET_ROOT_PATH;
  return this.generateKeyForPath(pathRoot);
}
module.exports = getHardenedFeaturePath;
