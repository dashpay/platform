const Dashcore = require('@dashevo/dashcore-lib');
const {
  BIP44_LIVENET_ROOT_PATH, BIP44_TESTNET_ROOT_PATH,
} = require('../CONSTANTS');
/**
 * Will return a root account path
 * @param network - default : 'testnet'
 * @param accountIndex - default : 0
 * @return {string} - BIP44 Path to account
 */
const getBIP44Path = function (network, accountIndex = 0) {
  return (network === Dashcore.Networks.livenet.toString())
    ? `${BIP44_LIVENET_ROOT_PATH}/${accountIndex}'`
    : `${BIP44_TESTNET_ROOT_PATH}/${accountIndex}'`;
};

module.exports = getBIP44Path;
