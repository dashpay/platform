import dapiGrpc from '@dashevo/dapi-grpc';
import GetTransactionResponse from './GetTransactionResponse.js';
import InvalidResponseError from '../../platform/response/errors/InvalidResponseError.js';

const {
  v0: {
    GetTransactionRequest,
    CorePromiseClient,
  },
} = dapiGrpc;

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getTransaction}
 */
function getTransactionFactory(grpcTransport) {
  /**
   * Get Transaction by ID
   * @typedef {getTransaction}
   * @param {string} id
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<null|Uint8Array>}
   */
  async function getTransaction(id, options = {}) {
    const getTransactionRequest = new GetTransactionRequest();
    getTransactionRequest.setId(id);

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const response = await grpcTransport.request(
          CorePromiseClient,
          'getTransaction',
          getTransactionRequest,
          options,
        );

        return GetTransactionResponse.createFromProto(response);
      } catch (e) {
        if (e instanceof InvalidResponseError) {
          lastError = e;
        } else {
          throw e;
        }
      }
    }

    // If we made it past the cycle it means that the retry didn't work,
    // and we're throwing the last error encountered
    throw lastError;
  }

  return getTransaction;
}

export default getTransactionFactory;
