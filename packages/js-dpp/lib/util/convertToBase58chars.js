/**
 * @param {string} string
 * @return {string}
 */
function convertToBase58chars(string) {
  return string
    .replace('0', 'o')
    .replace('l', '1');
}

module.exports = convertToBase58chars;
