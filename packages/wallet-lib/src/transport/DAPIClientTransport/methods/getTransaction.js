import dashcore from '@dashevo/dashcore-lib';
const { Transaction } = dashcore;
import NotFoundError from '@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError.js';
import { is } from '../../../utils/index.js';
import logger from '../../../logger/index.js';

/**
 * @param {string} txid
 * @returns {Promise<null|Transaction>}
 */
export default async function getTransaction(txid) {
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
