import validateHex from './validateHex.js';

/**
 *
 * @param {string} value
 * @returns {boolean}
 */
export default function validateTxHex(value) {
  return validateHex(value) && value.length === 64;
}
