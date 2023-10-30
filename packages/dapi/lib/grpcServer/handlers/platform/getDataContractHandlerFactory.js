const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetDataContractResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {DriveClient} driveClient
 *
 * @returns {getDataContractHandler}
 */
function getDataContractHandlerFactory(driveClient) {
  /**
   * @typedef getDataContractHandler
   *
   * @param {Object} call
   *
   * @returns {Promise<GetDataContractResponse>}
   */
  async function getDataContractHandler(call) {
    const { request } = call;

    if (request.getV0().getId() === null) {
      throw new InvalidArgumentGrpcError('id is not specified');
    }

    const dataContractResponseBuffer = await driveClient.fetchDataContract(request);

    return GetDataContractResponse.deserializeBinary(dataContractResponseBuffer);
  }

  return getDataContractHandler;
}

module.exports = getDataContractHandlerFactory;
