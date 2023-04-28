/**
 *
 * @param {string} value
 * @returns {boolean}
 */
function validateHex(value) {
  return Boolean(value.match(/^[0-9a-fA-F]+$/));
}

module.exports = validateHex;
