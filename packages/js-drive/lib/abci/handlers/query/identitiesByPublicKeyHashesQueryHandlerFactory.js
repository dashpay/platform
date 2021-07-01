const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const {
  v0: {
    GetIdentitiesByPublicKeyHashesResponse,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');

const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');

/**
 *
 * @param {PublicKeyToIdentityIdStoreRepository} previousPublicKeyToIdentityIdRepository
 * @param {IdentityStoreRepository} previousIdentityRepository
 * @param {number} maxIdentitiesPerRequest
 * @param {RootTree} previousRootTree
 * @param {IdentitiesStoreRootTreeLeaf} previousIdentitiesStoreRootTreeLeaf
 * @param {createQueryResponse} createQueryResponse
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContext} previousBlockExecutionContext
 * @return {identitiesByPublicKeyHashesQueryHandler}
 */
function identitiesByPublicKeyHashesQueryHandlerFactory(
  previousPublicKeyToIdentityIdRepository,
  previousIdentityRepository,
  maxIdentitiesPerRequest,
  previousRootTree,
  previousIdentitiesStoreRootTreeLeaf,
  createQueryResponse,
  blockExecutionContext,
  previousBlockExecutionContext,
) {
  /**
   * @typedef identitiesByPublicKeyHashesQueryHandler
   * @param {Object} params
   * @param {Object} data
   * @param {Buffer[]} data.publicKeyHashes
   * @param {Object} request
   * @param {boolean} [request.prove]
   * @return {Promise<ResponseQuery>}
   */
  async function identitiesByPublicKeyHashesQueryHandler(params, { publicKeyHashes }, request) {
    if (publicKeyHashes && publicKeyHashes.length > maxIdentitiesPerRequest) {
      throw new InvalidArgumentAbciError(
        `Maximum number of ${maxIdentitiesPerRequest} requested items exceeded.`, {
          maxIdentitiesPerRequest,
        },
      );
    }

    // There is no signed state (current committed block height less then 2)
    if (blockExecutionContext.isEmpty() || previousBlockExecutionContext.isEmpty()) {
      const response = new GetIdentitiesByPublicKeyHashesResponse();

      response.setIdentitiesList(publicKeyHashes.map(() => Buffer.alloc(0)));

      response.setMetadata(new ResponseMetadata());

      return new ResponseQuery({
        value: response.serializeBinary(),
      });
    }

    const isProofRequested = request.prove === 'true';

    const response = createQueryResponse(GetIdentitiesByPublicKeyHashesResponse, isProofRequested);

    const identityIds = await Promise.all(
      publicKeyHashes.map((publicKeyHash) => (
        previousPublicKeyToIdentityIdRepository.fetch(publicKeyHash)
      )),
    );

    if (isProofRequested) {
      const proof = response.getProof();

      const {
        rootTreeProof,
        storeTreeProof,
      } = previousRootTree.getFullProof(
        previousIdentitiesStoreRootTreeLeaf,
        identityIds.filter(Boolean).map((identityId) => identityId.toBuffer()),
      );

      proof.setRootTreeProof(rootTreeProof);
      proof.setStoreTreeProof(storeTreeProof);
    } else {
      const identityBuffers = await Promise.all(
        identityIds.map(async (identityId) => {
          if (!identityId) {
            return Buffer.alloc(0);
          }

          const identity = await previousIdentityRepository.fetch(identityId);

          return identity.toBuffer();
        }),
      );

      response.setIdentitiesList(identityBuffers);
    }

    return new ResponseQuery({
      value: response.serializeBinary(),
    });
  }

  return identitiesByPublicKeyHashesQueryHandler;
}

module.exports = identitiesByPublicKeyHashesQueryHandlerFactory;
