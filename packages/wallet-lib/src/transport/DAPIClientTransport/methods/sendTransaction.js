const { is } = require('../../../utils');
const logger = require('../../../logger');

module.exports = async function sendTransaction(serializedTransaction) {
  logger.silly('DAPIClientTransport.sendTransaction');
  if (!is.string(serializedTransaction)) throw new Error('Received an invalid rawtx');
  return this.client.core.broadcastTransaction(Buffer.from(serializedTransaction, 'hex'));
};
