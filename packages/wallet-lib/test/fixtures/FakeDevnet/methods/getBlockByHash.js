const { Block } = require('@dashevo/dashcore-lib');

module.exports = async function getBlockByHash(hash) {
  return new Block(Buffer.from(this.blocks.hashes[hash], 'hex'));
};
