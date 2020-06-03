const logger = require('../../../../logger');

module.exports = async function getBlockHeaderByHeight(blockHeight) {
  logger.silly(`DAPIClient.getBlockHeaderByHeight[${blockHeight}]`);
  return (await this.getBlockByHeight(blockHeight)).header;
};
