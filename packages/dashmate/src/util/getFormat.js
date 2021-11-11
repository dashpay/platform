/**
 * Extracts format from flags
 *
 * @param {Object} flags
 * @returns {false|string}
 */
function getFormat(flags) {
  return flags && flags.format;
}

module.exports = getFormat;
