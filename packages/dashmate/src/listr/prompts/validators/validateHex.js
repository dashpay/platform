/**
 *
 * @param {string} value
 * @returns {boolean}
 */
export function validateHex(value) {
  return Boolean(value.match(/^[0-9a-fA-F]+$/));
}
