const { Transaction } = require('@dashevo/dashcore-lib');
const NotFoundError = require('@dashevo/dapi-client/lib/methods/errors/NotFoundError');
const { is } = require('../../../utils');
const logger = require('../../../logger');

/**
 * @param {string} txid
 * @returns {Promise<null|Transaction>}
 */
module.exports = async function getTransaction(txid) {
  logger.silly(`DAPIClient.getTransaction[${txid}]`);
  if (!is.txid(txid)) {
    throw new Error(`Received an invalid txid to fetch : ${txid}`);
  }

  try {
    const response = await this.client.core.getTransaction(txid);
    return new Transaction(response.getTransaction());
  } catch (e) {
    if (e instanceof NotFoundError) {
      return null;
    }

    throw e;
  }
};
