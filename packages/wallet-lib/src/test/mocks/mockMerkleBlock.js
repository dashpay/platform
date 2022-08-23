const { MerkleBlock } = require('@dashevo/dashcore-lib');

const mockMerkleBlock = (txHashes) => {
  const merkleBlock = new MerkleBlock(Buffer.from([0, 0, 0, 32, 61, 11, 102, 108, 38, 155, 164, 49, 91, 246, 141, 178, 126, 155, 13, 118, 248, 83, 250, 15, 206, 21, 102, 65, 104, 183, 243, 167, 235, 167, 60, 113, 140, 110, 120, 87, 208, 191, 240, 19, 212, 100, 228, 121, 192, 125, 143, 44, 226, 9, 95, 98, 51, 25, 139, 172, 175, 27, 205, 201, 158, 85, 37, 8, 72, 52, 36, 95, 255, 255, 127, 32, 2, 0, 0, 0, 1, 0, 0, 0, 1, 140, 110, 120, 87, 208, 191, 240, 19, 212, 100, 228, 121, 192, 125, 143, 44, 226, 9, 95, 98, 51, 25, 139, 172, 175, 27, 205, 201, 158, 85, 37, 8, 1, 1]));
  merkleBlock.hashes = txHashes.map((hash) => Buffer.from(hash, 'hex').reverse().toString('hex'));
  return merkleBlock;
};

module.exports = mockMerkleBlock;
