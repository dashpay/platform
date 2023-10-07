/**
 * @param {string} input
 * @return {string}
 */
function convertToHomographSafeChars(input) {
  return input.toLowerCase().replace(/[oli]/g, (match) => {
    if (match === 'o') {
      return '0';
    }

    if (match === 'l' || match === 'i') {
      return '1';
    }

    return match;
  });
}

module.exports = convertToHomographSafeChars;
