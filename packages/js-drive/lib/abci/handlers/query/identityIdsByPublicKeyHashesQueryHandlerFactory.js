const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const {
  v0: {
    GetIdentityIdsByPublicKeyHashesResponse,
  },
} = require('@dashevo/dapi-grpc');

const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');

/**
 *
 * @param {PublicKeyToIdentityIdStoreRepository} previousPublicKeyToIdentityIdRepository
 * @param {number} maxIdentitiesPerRequest
 * @param {RootTree} previousRootTree
 * @param {PublicKeyToIdentityIdStoreRootTreeLeaf} previousPublicKeyToIdentityIdStoreRootTreeLeaf
 * @param {createQueryResponse} createQueryResponse
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContext} previousBlockExecutionContext
 * @return {identityIdsByPublicKeyHashesQueryHandler}
 */
function identityIdsByPublicKeyHashesQueryHandlerFactory(
  previousPublicKeyToIdentityIdRepository,
  maxIdentitiesPerRequest,
  previousRootTree,
  previousPublicKeyToIdentityIdStoreRootTreeLeaf,
  createQueryResponse,
  blockExecutionContext,
  previousBlockExecutionContext,
) {
  /**
   * @typedef identityIdsByPublicKeyHashesQueryHandler
   * @param {Object} params
   * @param {Object} data
   * @param {Buffer[]} data.publicKeyHashes
   * @param {Object} request
   * @param {boolean} [request.prove]
   * @return {Promise<ResponseQuery>}
   */
  async function identityIdsByPublicKeyHashesQueryHandler(params, { publicKeyHashes }, request) {
    if (publicKeyHashes && publicKeyHashes.length > maxIdentitiesPerRequest) {
      throw new InvalidArgumentAbciError(
        `Maximum number of ${maxIdentitiesPerRequest} requested items exceeded.`, {
          maxIdentitiesPerRequest,
        },
      );
    }

    // There is no signed state (current committed block height less then 2)
    if (blockExecutionContext.isEmpty() || previousBlockExecutionContext.isEmpty()) {
      const response = new GetIdentityIdsByPublicKeyHashesResponse();

      response.setIdentityIdsList(publicKeyHashes.map(() => Buffer.alloc(0)));

      return new ResponseQuery({
        value: response.serializeBinary(),
      });
    }

    const isProofRequested = request.prove === 'true';

    const response = createQueryResponse(GetIdentityIdsByPublicKeyHashesResponse, isProofRequested);

    const identityIds = await Promise.all(
      publicKeyHashes.map(async (publicKeyHash) => (
        previousPublicKeyToIdentityIdRepository.fetch(publicKeyHash)
      )),
    );

    if (isProofRequested) {
      const proof = response.getProof();

      const {
        rootTreeProof,
        storeTreeProof,
      } = previousRootTree.getFullProof(
        previousPublicKeyToIdentityIdStoreRootTreeLeaf,
        identityIds.filter(Boolean).map((identityId) => identityId.toBuffer()),
      );

      proof.setRootTreeProof(rootTreeProof);
      proof.setStoreTreeProof(storeTreeProof);
    } else {
      const identityIdBuffers = identityIds.map((identityId) => {
        if (!identityId) {
          return Buffer.alloc(0);
        }

        return identityId.toBuffer();
      });

      response.setIdentityIdsList(identityIdBuffers);
    }

    return new ResponseQuery({
      value: response.serializeBinary(),
    });
  }

  return identityIdsByPublicKeyHashesQueryHandler;
}

module.exports = identityIdsByPublicKeyHashesQueryHandlerFactory;
