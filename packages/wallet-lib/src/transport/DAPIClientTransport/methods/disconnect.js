import logger from '../../../logger/index.js';

export default async function disconnect() {
  logger.silly('DAPIClientTransport.disconnect');

  return this.client.disconnect();
};
