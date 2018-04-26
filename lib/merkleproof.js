const utils = require('./utils');
const bmp = require('bitcoin-merkle-proof');

function getMerkleProofs(localMerkleRoot) {
  if (utils.getCorrectedHash(message.merkleBlock.header.merkleRoot)
    !== utils.getCorrectedHash(localMerkleRoot)) {
    return false;
  }

  try {
    bmp.verify({
      flags: message.merkleBlock.flags,
      hashes: message.merkleBlock.hashes.map(h => Buffer.from(h, 'hex')),
      numTransactions: message.merkleBlock.numTransactions,
      merkleRoot: message.merkleBlock.header.merkleRoot,
    });
  } catch (e) {
    // todo
  }
}

function isIncluded(localBlock, txHash) {
  return getMerkleProofs(localBlock.hash, localBlock.merkleRoot)
    .then((proofs) => {
      if (proofs.map(p => utils.getCorrectedHash(p)).includes(txHash)) {
        // coinbase tx only so
        // merkle root matches txHash so can do this check here
        // console.log('SPV VALIDTION SUCCESS')
        return true;
      }
      return false;
    })
    .catch(err => err);
}


module.exports = isIncluded;
