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
 * @param {DriveStateRepository} driveStateRepository
 * @param {handleAbciResponseError} handleAbciResponseError
 *
 * @returns {getDataContractHandler}
 */
function getDataContractHandlerFactory(driveStateRepository, handleAbciResponseError) {
  /**
   * @typedef getDataContractHandler
   *
   * @param {Object} call
   *
   * @returns {Promise<GetDocumentsResponse>}
   */
  async function getDataContractHandler(call) {
    const { request } = call;
    const id = request.getId();

    if (id === null) {
      throw new InvalidArgumentGrpcError('id is not specified');
    }

    let dataContractBuffer;
    try {
      dataContractBuffer = await driveStateRepository.fetchDataContract(id);
    } catch (e) {
      if (e instanceof AbciResponseError) {
        handleAbciResponseError(e);
      }
      throw e;
    }

    const response = new GetDataContractResponse();

    response.setDataContract(dataContractBuffer);

    return response;
  }

  return getDataContractHandler;
}

module.exports = getDataContractHandlerFactory;
