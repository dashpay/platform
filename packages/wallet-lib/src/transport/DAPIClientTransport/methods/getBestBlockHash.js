import logger from '../../../logger/index.js';

export default async function getBestBlockHash() {
  logger.silly('DAPIClientTransport.getBestBlockHash');

  return this.client.core.getBestBlockHash();
};
