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
  },
} = require('@dashevo/dapi-grpc');

const NotFoundAbciError = require('../../errors/NotFoundAbciError');

/**
 *
 * @param {IdentityStoreRepository} previousIdentityRepository
 * @param {RootTree} previousRootTree
 * @param {IdentitiesStoreRootTreeLeaf} previousIdentitiesStoreRootTreeLeaf
 * @param {createQueryResponse} createQueryResponse
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContext} previousBlockExecutionContext
 * @return {identityQueryHandler}
 */
function identityQueryHandlerFactory(
  previousIdentityRepository,
  previousRootTree,
  previousIdentitiesStoreRootTreeLeaf,
  createQueryResponse,
  blockExecutionContext,
  previousBlockExecutionContext,
) {
  /**
   * @typedef identityQueryHandler
   * @param {Object} params
   * @param {Object} options
   * @param {Buffer} options.id
   * @param {Object} request
   * @param {boolean} [request.prove]
   * @return {Promise<ResponseQuery>}
   */
  async function identityQueryHandler(params, { id }, request) {
    // There is no signed state (current committed block height less then 2)
    if (blockExecutionContext.isEmpty() || previousBlockExecutionContext.isEmpty()) {
      throw new NotFoundAbciError('Identity not found');
    }

    const isProofRequested = request.prove === 'true';

    const response = createQueryResponse(GetIdentityResponse, isProofRequested);

    const identity = await previousIdentityRepository.fetch(id);

    if (!identity) {
      throw new NotFoundAbciError('Identity not found');
    }

    const identityBuffer = identity.toBuffer();

    if (isProofRequested) {
      const proof = response.getProof();

      const {
        rootTreeProof,
        storeTreeProof,
      } = previousRootTree.getFullProof(
        previousIdentitiesStoreRootTreeLeaf,
        [id],
      );

      proof.setRootTreeProof(rootTreeProof);
      proof.setStoreTreeProof(storeTreeProof);
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
