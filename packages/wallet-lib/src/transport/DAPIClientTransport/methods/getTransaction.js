const { Transaction } = require('@dashevo/dashcore-lib');
const { is } = require('../../../utils');
const logger = require('../../../logger');

module.exports = async function getTransaction(txid) {
  logger.silly(`DAPIClient.getTransaction[${txid}]`);
  if (!is.txid(txid)) throw new Error(`Received an invalid txid to fetch : ${txid}`);
  return new Transaction(await this.client.core.getTransaction(txid));
};
