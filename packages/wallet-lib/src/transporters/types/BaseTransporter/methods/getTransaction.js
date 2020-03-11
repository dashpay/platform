const { is } = require('../../../../utils');

module.exports = async function getTransaction(txid) {
  if (!is.txid(txid)) throw new Error(`Received an invalid txid to fetch : ${txid}`);
  throw new Error('Not implemented');
};
