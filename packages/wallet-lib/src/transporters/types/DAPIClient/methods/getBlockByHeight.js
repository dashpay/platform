const { Block } = require('@dashevo/dashcore-lib');

module.exports = async function getBlockByHeight(height) {
  return new Block(await this.client.getBlockByHeight(height));
};
