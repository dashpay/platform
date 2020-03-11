const { Block } = require('@dashevo/dashcore-lib');

module.exports = async function getBlockByHash(blockHash) {
  return new Block(await this.client.getBlockByHash(blockHash));
};
