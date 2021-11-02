const hashLength = 32;

module.exports = function getStoreProofData(storeProof) {
  const buf = storeProof;
  const hashes = [];
  const keyValueHashes = [];
  const values = [];
  const keyValues = {};

  let x = 0;
  while (x < buf.length) {
    const type = buf.readUInt8(x);
    x += 1;

    switch (type) {
      case 0x01: { // Hash
        hashes.push(buf.slice(x, x + hashLength));
        x += hashLength;
        break;
      }

      case 0x02: { // Key/value hash
        keyValueHashes.push(buf.slice(x, x + hashLength));
        x += hashLength;
        break;
      }

      case 0x03: { // Key / Value
        const keySize = buf.readUInt8(x);
        x += 1;
        const key = buf.toString('hex', x, x + keySize);
        x += keySize;

        const valueSize = buf.readUInt16BE(x);
        x += 2;

        // Value
        const valueHex = buf.toString('hex', x, x + valueSize);
        const valueBuffer = Buffer.from(valueHex, 'hex');
        x += valueSize;

        keyValues[key] = valueBuffer;
        values.push(valueBuffer);
        break;
      }

      case 0x10: // Parent
        break;

      case 0x11: // Child
        break;

      default:
        throw new Error(`Unknown type: ${type.toString(16)}`);
    }
  }

  return {
    hashes, keyValueHashes, values, keyValues,
  };
};
