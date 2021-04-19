const logger = require('../../../logger');

module.exports = async function getBestBlockHeight() {
  logger.silly('DAPIClientTransport.getBestBlockHeight');

  // Previously we would have done getBlock(hash).height
  const { chain: { blocksCount } } = await this.getStatus();

  return blocksCount;
};
