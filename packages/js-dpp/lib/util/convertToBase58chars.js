/**
 * @param {string} input
 * @return {string}
 */
function convertToBase58chars(input) {
  return input.replace(/[0l]/g, (match) => {
    if (match === '0') {
      return 'o';
    }

    if (match === 'l') {
      return '1';
    }

    return match;
  });
}

module.exports = convertToBase58chars;
