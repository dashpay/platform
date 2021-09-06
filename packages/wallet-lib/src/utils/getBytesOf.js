const { Script, Address } = require('@dashevo/dashcore-lib');

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
module.exports = getBytesOf;
