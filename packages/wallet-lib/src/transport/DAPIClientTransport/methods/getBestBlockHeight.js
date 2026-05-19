import logger from '../../../logger/index.js';

export default async function getBestBlockHeight() {
  logger.silly('DAPIClientTransport.getBestBlockHeight');

  return this.client.core.getBestBlockHeight();
};
