const { Address } = require('@dashevo/dashcore-lib');

/**
 * @param {string} value
 * @param {string} network
 * @returns {boolean}
 */
function validateAddress(value, network) {
  try {
    Address(value, network);
  } catch (e) {
    return false;
  }

  return true;
}

module.exports = validateAddress;
