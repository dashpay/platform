const fs = require('fs');

/**
 * @param {string} value
 * @returns {boolean}
 */
function validateFileExists(value) {
  try {
    // eslint-disable-next-line no-bitwise
    fs.accessSync(value, fs.constants.R_OK | fs.constants.W_OK);

    return true;
  } catch (e) {
    return false;
  }
}

module.exports = validateFileExists;
