const { is } = require('../../../../utils');
const logger = require('../../../../logger');

module.exports = async function sendTransaction(serializedTransaction) {
  logger.silly('DAPIClientWrapper.sendTransaction');
  if (!is.string(serializedTransaction)) throw new Error('Received an invalid rawtx');
  return this.client.sendTransaction(Buffer.from(serializedTransaction, 'hex'));
};
