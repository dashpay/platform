/**
 * @param {string} input
 * @return {string}
 */
function convertToBase58chars(input) {
  return input.replace(/[oi]/g, (match) => {
    if (match === 'o') {
      return '0';
    }

    if (match === 'i') {
      return '1';
    }

    return match;
  });
}

module.exports = convertToBase58chars;
