const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetDataContractHistoryResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {DriveClient} driveClient
 *
 * @returns {getDataContractHistoryHandler}
 */
function getDataContractHistoryHandlerFactory(driveClient) {
  /**
     * @typedef getDataContractHistoryHandler
     *
     * @param {Object} call
     *
     * @return {Promise<GetDataContractHistoryResponse>}
     */
  async function getDataContractHistoryHandler(call) {
    const { request } = call;

    if (request.getV0().getId() === null) {
      throw new InvalidArgumentGrpcError('id is not specified');
    }

    const dataContractHistoryResponseBuffer = await driveClient.fetchDataContractHistory(request);

    return GetDataContractHistoryResponse.deserializeBinary(dataContractHistoryResponseBuffer);
  }

  return getDataContractHistoryHandler;
}

module.exports = getDataContractHistoryHandlerFactory;
