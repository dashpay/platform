import logger from '../../../logger/index.js';

export default async function getBlockchainStatus() {
  logger.silly('DAPIClientTransport.getBlockchainStatus');

  return this.client.core.getBlockchainStatus();
};
