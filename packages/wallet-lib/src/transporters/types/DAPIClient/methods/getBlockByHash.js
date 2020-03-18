const { Block } = require('@dashevo/dashcore-lib');
const logger = require('../../../../logger');

module.exports = async function getBlockByHash(blockHash) {
  logger.silly(`DAPIClient.getBlockByHash[${blockHash}]`);
  return new Block(await this.client.getBlockByHash(blockHash));
};
