const { Script, Address } = require('@dashevo/dashcore-lib');
const { DUFFS_PER_DASH } = require('../CONSTANTS');

function dashToDuffs(dash) {
  if (dash === undefined || dash.constructor.name !== Number.name) {
    throw new Error('Can only convert a number');
  }
  return parseInt((dash * DUFFS_PER_DASH).toFixed(0), 10);
}
function duffsToDash(duffs) {
  if (duffs === undefined || duffs.constructor.name !== Number.name) {
    throw new Error('Can only convert a number');
  }
  return duffs / DUFFS_PER_DASH;
}

function hasProp(obj, prop) {
  if (!obj) return false;
  if (Array.isArray(obj)) {
    return obj.includes(prop);
  }
  return {}.hasOwnProperty.call(obj, prop);
}

/**
 *
 * @param {object} obj
 * @param {string} methodName
 * @return {boolean}
 */
function hasMethod(obj, methodName) {
  return typeof obj[methodName] === 'function';
}

function getBytesOf(elem, type) {
  let BASE_BYTES = 0;
  let SCRIPT_BYTES = 0;

  switch (type) {
    case 'utxo':
      BASE_BYTES = 32 + 4 + 1 + 4;
      SCRIPT_BYTES = Buffer.from(elem.script, 'hex').length;
      return BASE_BYTES + SCRIPT_BYTES;
    case 'output':
      BASE_BYTES = 8 + 1;
      SCRIPT_BYTES = Script(new Address(elem.address)).toBuffer().length;
      return BASE_BYTES + SCRIPT_BYTES;
    default:
      return false;
  }
}
module.exports = {
  dashToDuffs, duffsToDash, getBytesOf, hasProp, hasMethod,
};
