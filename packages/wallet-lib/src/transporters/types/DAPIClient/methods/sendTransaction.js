const { is } = require('../../../../utils');

module.exports = async function sendTransaction(serializedTransaction) {
  if (!is.string(serializedTransaction)) throw new Error('Received an invalid rawtx');
  return this.client.sendTransaction(Buffer.from(serializedTransaction, 'hex'));
};
