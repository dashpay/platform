const logger = require('../../../../logger');

module.exports = async function getBestBlockHeight() {
  logger.silly('DAPIClient.getBestBlockHeight');
  // Previously we would have done getBlock(hash).height
  return (await this.getStatus()).blocks;
};
