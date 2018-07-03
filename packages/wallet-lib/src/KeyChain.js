const Dashcore = require('@dashevo/dashcore-lib');

/**
 * Return newly derived private key
 * @return {string}
 */
const getNewPrivateKey = (HDKey = new Dashcore.HDPrivateKey(), derivationPath = 'm/1') => String(HDKey.derive(derivationPath));

module.exports = { getNewPrivateKey };
