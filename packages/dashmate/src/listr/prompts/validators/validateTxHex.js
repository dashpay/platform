import {validateHex} from './validateHex';

/**
 *
 * @param {string} value
 * @returns {boolean}
 */
export function validateTxHex(value) {
  return validateHex(value) && value.length === 64;
}
