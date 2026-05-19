import logger from '../../../logger/index.js';

export default async function getBestBlockHeader() {
  logger.silly('DAPIClientTransport.getBestBlockHeader');

  return this.getBlockHeaderByHash(await this.getBestBlockHash());
};
