const {
  MerkleBlock,
} = require('@dashevo/dashcore-lib');

const getHeightFromMerkleBlockBuffer = async (client, merkleBlockBuffer) => {
  // FIXME: MerkleBlock do not accept hex.
  const merkleBlock = new MerkleBlock(Buffer.from(merkleBlockBuffer));
  const prevHash = merkleBlock.header.prevHash.reverse().toString('hex');

  const prevBlock = await client.getBlockByHash(prevHash);
  try {
    const prevBlockHeight = prevBlock.transactions[0].extraPayload.height;
    return prevBlockHeight + 1;
  } catch (e) {
    const prevBlockHeight = prevBlock.transactions[1].extraPayload.height;
    return prevBlockHeight + 1;
  }
};
module.exports = getHeightFromMerkleBlockBuffer;
