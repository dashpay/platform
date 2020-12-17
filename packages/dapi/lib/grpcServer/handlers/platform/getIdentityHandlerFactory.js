const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentityResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const AbciResponseError = require('../../../errors/AbciResponseError');

/**
 * @param {DriveStateRepository} driveStateRepository
 * @param {handleAbciResponseError} handleAbciResponseError
 *
 * @returns {getIdentityHandler}
 */
function getIdentityHandlerFactory(driveStateRepository, handleAbciResponseError) {
  /**
   * @typedef getIdentityHandler
   *
   * @param {Object} call
   *
   * @return {Promise<GetIdentityResponse>}
   */
  async function getIdentityHandler(call) {
    const { request } = call;

    const id = request.getId();

    if (!id) {
      throw new InvalidArgumentGrpcError('id is not specified');
    }

    const prove = request.getProve();

    let identityBuffer;
    let proofObject;

    try {
      ({ data: identityBuffer, proof: proofObject } = await driveStateRepository
        .fetchIdentity(id, prove));
    } catch (e) {
      if (e instanceof AbciResponseError) {
        handleAbciResponseError(e);
      }
      throw e;
    }

    const response = new GetIdentityResponse();

    response.setIdentity(identityBuffer);

    if (prove === true) {
      const proof = new Proof();
      proof.setRootTreeProof(proofObject.rootTreeProof);
      proof.setStoreTreeProof(proofObject.storeTreeProof);

      response.setProof(proof);
    }

    return response;
  }

  return getIdentityHandler;
}

module.exports = getIdentityHandlerFactory;
