import { Address } from '@dashevo/dashcore-lib';

/**
 * @param {string} value
 * @param {string} network
 * @returns {boolean}
 */
export function validateAddress(value, network) {
  try {
    Address(value, network);
  } catch (e) {
    return false;
  }

  return true;
}
