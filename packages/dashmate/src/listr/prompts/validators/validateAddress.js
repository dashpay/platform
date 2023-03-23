const { Address } = require('@dashevo/dashcore-lib');

function validateAddress(value, network) {
  try {
    Address(value, network);
  } catch (e) {
    return false;
  }

  return true;
}

module.exports = validateAddress;
