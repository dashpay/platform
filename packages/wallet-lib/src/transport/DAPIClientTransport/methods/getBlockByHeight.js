const { Block } = require('@dashevo/dashcore-lib');
const logger = require('../../../logger');

module.exports = async function getBlockByHeight(height) {
  logger.silly(`DAPIClient.getBlockByHeight[${height}]`);

  return new Block(await this.client.core.getBlockByHeight(height));
};
