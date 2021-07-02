const {
  v0: {
    GetTransactionRequest,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const GetTransactionResponse = require('./GetTransactionResponse');
const NotFoundError = require('../../errors/NotFoundError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getTransaction}
 */
function getTransactionFactory(grpcTransport) {
  /**
   * Get Transaction by ID
   *
   * @typedef {getTransaction}
   * @param {string} id
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<null|Buffer>}
   */
  async function getTransaction(id, options = {}) {
    const getTransactionRequest = new GetTransactionRequest();
    getTransactionRequest.setId(id);

    let response;
    try {
      response = await grpcTransport.request(
        CorePromiseClient,
        'getTransaction',
        getTransactionRequest,
        options,
      );
    } catch (e) {
      if (e.code === grpcErrorCodes.NOT_FOUND) {
        throw new NotFoundError(`Transaction ${id} is not found`);
      }

      throw e;
    }

    return GetTransactionResponse.createFromProto(response);
  }

  return getTransaction;
}

module.exports = getTransactionFactory;
