const { Transaction } = require('@dashevo/dashcore-lib');
const NotFoundError = require('@dashevo/dapi-client/lib/errors/response/NotFoundError');
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
    const {
      height,
      instantLocked,
      chainLocked,
    } = response;

    return {
      transaction: new Transaction(response.getTransaction()),
      blockHash: response.getBlockHash().toString('hex'),
      height,
      instantLocked,
      chainLocked,
    };
  } catch (e) {
    if (e instanceof NotFoundError) {
      return null;
    }

    throw e;
  }
};
