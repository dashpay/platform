const { Address } = require('@dashevo/dashcore-lib');
const bs58 = require('bs58');

function validateAddressHex(value, network) {
  if (value.length === 0) {
    return false;
  }

  const base58value = bs58.encode(Buffer.from(value, 'hex'));

  try {
    Address(base58value, network);
  } catch (e) {
    return false;
  }

  return true;
}

module.exports = validateAddressHex;
