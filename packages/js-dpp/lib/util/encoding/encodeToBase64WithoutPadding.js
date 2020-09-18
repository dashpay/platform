/**
 * Encode buffer to base64 string without padding
 *
 * @param {Buffer} buffer
 *
 * @returns {string}
 */
function encodeToBase64WithoutPadding(buffer) {
  return buffer.toString('base64').replace(/=/g, '');
}

module.exports = encodeToBase64WithoutPadding;
