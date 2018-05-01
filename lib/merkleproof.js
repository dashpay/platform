const bmp = require('bitcoin-merkle-proof');

function validateMerkleBlock(merkleBlock, txHash) {
  try {
    return bmp.verify({
      flags: merkleBlock.flags,
      hashes: merkleBlock.hashes.map(h => Buffer.from(h, 'hex')),
      include: [txHash],
      numTransactions: merkleBlock.numTransactions,
      merkleRoot: Buffer.from(merkleBlock.header.merkleRoot, 'hex'), // Buffer.from(merkleBlock.header.merkleRoot, 'hex'),
    });
  } catch (e) {
    return false;
  }
}

function isValid(merkleBlock, localHeader, txHash) {
  if (localHeader.hash !== merkleBlock.header.hash) {
    return false;
  }

  // todo: temp workaround
  const clone = JSON.parse(JSON.stringify(merkleBlock));
  clone.header.merkleRoot = Buffer.from(merkleBlock.header.merkleRoot, 'hex').reverse().toString('hex');

  return validateMerkleBlock(clone, txHash);
}

module.exports = isValid;
