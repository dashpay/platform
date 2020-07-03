const logger = require('../../../logger');

module.exports = async function getBestBlock() {
  logger.silly('FakeNet.getBestBlock');
  return this.getBlockByHash(await this.getBestBlockHash());
};
