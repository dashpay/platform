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
    const id = request.getId();
    const prove = request.getProve();

    if (id === null) {
      throw new InvalidArgumentGrpcError('id is not specified');
    }

    const dataContractResponseBuffer = await driveClient.fetchDataContract(id, prove);

    return GetDataContractResponse.deserializeBinary(dataContractResponseBuffer);
  }

  return getDataContractHandler;
}

module.exports = getDataContractHandlerFactory;
