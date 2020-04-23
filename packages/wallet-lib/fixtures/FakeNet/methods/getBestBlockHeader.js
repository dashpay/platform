const logger = require('../../../src/logger');

module.exports = async function getBestBlockHeader() {
  logger.silly('FakeNet.getBestBlockHeader');
  return this.getBlockHeaderByHash(await this.getBestBlockHash());
};
