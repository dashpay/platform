const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentitiesByPublicKeyHashesResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const AbciResponseError = require('../../../errors/AbciResponseError');

/**
 *
 * @param {DriveStateRepository} driveStateRepository
 * @param {handleAbciResponseError} handleAbciResponseError
 * @return {getIdentitiesByPublicKeyHashesHandler}
 */
function getIdentitiesByPublicKeyHashesHandlerFactory(
  driveStateRepository, handleAbciResponseError,
) {
  /**
   * @typedef getIdentitiesByPublicKeyHashesHandler
   * @param {Object} call
   * @return {Promise<GetIdentitiesByPublicKeyHashesResponse>}
   */
  async function getIdentitiesByPublicKeyHashesHandler(call) {
    const { request } = call;

    const publicKeyHashes = request.getPublicKeyHashesList();

    if (publicKeyHashes.length === 0) {
      throw new InvalidArgumentGrpcError('No public key hashes were provided');
    }

    const prove = request.getProve();

    let identities;
    let proofObject;
    try {
      ({ data: identities, proof: proofObject } = await driveStateRepository
        .fetchIdentitiesByPublicKeyHashes(publicKeyHashes, prove));
    } catch (e) {
      if (e instanceof AbciResponseError) {
        handleAbciResponseError(e);
      }
      throw e;
    }

    const response = new GetIdentitiesByPublicKeyHashesResponse();

    response.setIdentitiesList(
      identities,
    );

    if (prove === true) {
      const proof = new Proof();
      proof.setRootTreeProof(proofObject.rootTreeProof);
      proof.setStoreTreeProof(proofObject.storeTreeProof);

      response.setProof(proof);
    }

    return response;
  }

  return getIdentitiesByPublicKeyHashesHandler;
}

module.exports = getIdentitiesByPublicKeyHashesHandlerFactory;
