const validateHex = require('./validateHex');

/**
 *
 * @param {string} value
 * @returns {boolean}
 */
function validateTxHex(value) {
  return validateHex(value) && value.length === 64;
}

module.exports = validateTxHex;
