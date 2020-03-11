const { Transaction } = require('@dashevo/dashcore-lib');
const { is } = require('../../../../utils');

module.exports = async function getTransaction(txid) {
  if (!is.txid(txid)) throw new Error(`Received an invalid txid to fetch : ${txid}`);
  return new Transaction(await this.client.getTransaction(txid));
};
