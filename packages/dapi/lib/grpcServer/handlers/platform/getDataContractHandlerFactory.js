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

const AbciResponseError = require('../../../errors/AbciResponseError');

/**
 * @param {DriveClient} driveClient
 * @param {handleAbciResponseError} handleAbciResponseError
 *
 * @returns {getDataContractHandler}
 */
function getDataContractHandlerFactory(driveClient, handleAbciResponseError) {
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

    let dataContractResponseBuffer;
    try {
      dataContractResponseBuffer = await driveClient
        .fetchDataContract(id, prove);
    } catch (e) {
      if (e instanceof AbciResponseError) {
        handleAbciResponseError(e);
      }
      throw e;
    }

    return GetDataContractResponse.deserializeBinary(dataContractResponseBuffer);
  }

  return getDataContractHandler;
}

module.exports = getDataContractHandlerFactory;
