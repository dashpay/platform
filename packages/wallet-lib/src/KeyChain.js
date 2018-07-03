const Dashcore = require('@dashevo/dashcore-lib');

/**
 * Return newly derived private key
 * @return {string}
 */
const getNewPrivateKey = (HDKey = new Dashcore.HDPrivateKey(), derivationPath = 'm/1') => (HDKey.derive(derivationPath));

module.exports = { getNewPrivateKey };
