import { is } from '../../../utils/index.js';
import logger from '../../../logger/index.js';

export default async function sendTransaction(serializedTransaction) {
  logger.silly('DAPIClientTransport.sendTransaction');
  if (!is.string(serializedTransaction)) throw new Error('Received an invalid rawtx');
  return this.client.core.broadcastTransaction(Buffer.from(serializedTransaction, 'hex'));
};
