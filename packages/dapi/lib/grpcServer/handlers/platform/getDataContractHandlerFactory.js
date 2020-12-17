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
    Proof,
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
   * @returns {Promise<GetDataContractResponse>}
   */
  async function getDataContractHandler(call) {
    const { request } = call;
    const id = request.getId();
    const prove = request.getProve();

    if (id === null) {
      throw new InvalidArgumentGrpcError('id is not specified');
    }

    let dataContractBuffer;
    let proofObject;
    try {
      ({ data: dataContractBuffer, proof: proofObject } = await driveStateRepository
        .fetchDataContract(id, prove));
    } catch (e) {
      if (e instanceof AbciResponseError) {
        handleAbciResponseError(e);
      }
      throw e;
    }

    const response = new GetDataContractResponse();

    response.setDataContract(dataContractBuffer);

    if (prove === true) {
      const proof = new Proof();
      proof.setRootTreeProof(proofObject.rootTreeProof);
      proof.setStoreTreeProof(proofObject.storeTreeProof);

      response.setProof(proof);
    }

    return response;
  }

  return getDataContractHandler;
}

module.exports = getDataContractHandlerFactory;
