const logger = require('../../../logger');

module.exports = async function getBlockHeaderByHash(blockHash) {
  logger.silly(`DAPIClient.getBlockHeaderByHash[${blockHash}]`);

  return (await this.getBlockByHash(blockHash)).header;
};
