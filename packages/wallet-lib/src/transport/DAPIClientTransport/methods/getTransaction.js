const { Transaction } = require('@dashevo/dashcore-lib');
const NotFoundError = require('@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError');
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
      // dapi-client returns the raw transaction as a plain Uint8Array, but
      // dashcore-lib's Transaction reader relies on Buffer methods. Wrap to
      // Buffer here, consistent with the other raw-transaction call sites.
      transaction: new Transaction(Buffer.from(response.getTransaction())),
      // dapi-client returns blockHash as a plain Uint8Array, whose toString()
      // ignores the encoding argument. Wrap to Buffer to get hex semantics.
      blockHash: Buffer.from(response.getBlockHash()).toString('hex'),
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
