import logger from '../../../logger/index.js';

export default async function getBestBlock() {
  logger.silly('DAPIClientTransport.getBestBlock');

  return this.getBlockByHash(await this.getBestBlockHash());
};
