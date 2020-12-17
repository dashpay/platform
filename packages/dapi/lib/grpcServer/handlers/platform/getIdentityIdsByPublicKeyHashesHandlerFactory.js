const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentityIdsByPublicKeyHashesResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const AbciResponseError = require('../../../errors/AbciResponseError');

/**
 *
 * @param {DriveStateRepository} driveStateRepository
 * @param {handleAbciResponseError} handleAbciResponseError
 * @return {getIdentityIdsByPublicKeyHashesHandler}
 */
function getIdentityIdsByPublicKeyHashesHandlerFactory(
  driveStateRepository, handleAbciResponseError,
) {
  /**
   * @typedef getIdentityIdsByPublicKeyHashesHandler
   * @param {Object} call
   * @return {Promise<GetIdentityIdsByPublicKeyHashesResponse>}
   */
  async function getIdentityIdsByPublicKeyHashesHandler(call) {
    const { request } = call;

    const publicKeyHashes = request.getPublicKeyHashesList();

    if (publicKeyHashes.length === 0) {
      throw new InvalidArgumentGrpcError('No public key hashes were provided');
    }

    const prove = request.getProve();

    let identityIds;
    let proofObject;

    try {
      ({ data: identityIds, proof: proofObject } = await driveStateRepository
        .fetchIdentityIdsByPublicKeyHashes(
          publicKeyHashes,
          prove,
        ));
    } catch (e) {
      if (e instanceof AbciResponseError) {
        handleAbciResponseError(e);
      }
      throw e;
    }

    const response = new GetIdentityIdsByPublicKeyHashesResponse();

    response.setIdentityIdsList(
      identityIds,
    );

    if (prove === true) {
      const proof = new Proof();
      proof.setRootTreeProof(proofObject.rootTreeProof);
      proof.setStoreTreeProof(proofObject.storeTreeProof);

      response.setProof(proof);
    }

    return response;
  }

  return getIdentityIdsByPublicKeyHashesHandler;
}

module.exports = getIdentityIdsByPublicKeyHashesHandlerFactory;
