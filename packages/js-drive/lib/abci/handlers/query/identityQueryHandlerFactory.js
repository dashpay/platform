const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const {
  v0: {
    GetIdentityResponse,
    StoreTreeProofs,
  },
} = require('@dashevo/dapi-grpc');

const NotFoundAbciError = require('../../errors/NotFoundAbciError');

/**
 *
 * @param {IdentityStoreRepository} signedIdentityRepository
 * @param {createQueryResponse} createQueryResponse
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContextStack} blockExecutionContextStack
 * @return {identityQueryHandler}
 */
function identityQueryHandlerFactory(
  signedIdentityRepository,
  createQueryResponse,
  blockExecutionContext,
  blockExecutionContextStack,
) {
  /**
   * @typedef identityQueryHandler
   * @param {Object} params
   * @param {Object} options
   * @param {Buffer} options.id
   * @param {RequestQuery} request
   * @return {Promise<ResponseQuery>}
   */
  async function identityQueryHandler(params, { id }, request) {
    // There is no signed state (current committed block height less than 3)
    if (!blockExecutionContextStack.getLast()) {
      throw new NotFoundAbciError('Identity not found');
    }

    const response = createQueryResponse(GetIdentityResponse, request.prove);

    const identity = await signedIdentityRepository.fetch(id);

    let identityBuffer;
    if (!identity && !request.prove) {
      throw new NotFoundAbciError('Identity not found');
    } else if (identity) {
      identityBuffer = identity.toBuffer();
    }

    if (request.prove) {
      const proof = response.getProof();
      const storeTreeProofs = new StoreTreeProofs();

      const {
        rootTreeProof,
        storeTreeProof,
      } = previousRootTree.getFullProofForOneLeaf(
        previousIdentitiesStoreRootTreeLeaf,
        [id],
      );

      storeTreeProofs.setIdentitiesProof(storeTreeProof);

      proof.setRootTreeProof(rootTreeProof);
      proof.setStoreTreeProofs(storeTreeProofs);
    } else {
      response.setIdentity(identityBuffer);
    }

    return new ResponseQuery({
      value: response.serializeBinary(),
    });
  }

  return identityQueryHandler;
}

module.exports = identityQueryHandlerFactory;
