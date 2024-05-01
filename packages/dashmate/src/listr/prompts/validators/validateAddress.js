import DashCoreLib from '@dashevo/dashcore-lib';

const { Address } = DashCoreLib;
/**
 * @param {string} value
 * @param {string} network
 * @returns {boolean}
 */
export default function validateAddress(value, network) {
  try {
    Address(value, network);
  } catch (e) {
    return false;
  }

  return true;
}
