/**
 *
 * @param {string} value
 * @returns {boolean}
 */
export default function validateHex(value) {
  return Boolean(value.match(/^[0-9a-fA-F]+$/));
}
