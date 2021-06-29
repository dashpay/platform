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
 * @return {identityIdsByPublicKeyHashesQueryHandler}
 */
function identityIdsByPublicKeyHashesQueryHandlerFactory(
  previousPublicKeyToIdentityIdRepository,
  maxIdentitiesPerRequest,
  previousRootTree,
  previousPublicKeyToIdentityIdStoreRootTreeLeaf,
  createQueryResponse,
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

    const isProofRequested = request.prove === 'true';

    const response = createQueryResponse(GetIdentityIdsByPublicKeyHashesResponse, isProofRequested);

    const identityIdBuffers = await Promise.all(
      publicKeyHashes.map(async (publicKeyHash) => {
        const identityId = await previousPublicKeyToIdentityIdRepository.fetch(publicKeyHash);

        if (!identityId) {
          return Buffer.alloc(0);
        }

        return identityId.toBuffer();
      }),
    );

    if (isProofRequested) {
      const proof = response.getProof();

      const {
        rootTreeProof,
        storeTreeProof,
      } = previousRootTree.getFullProof(
        previousPublicKeyToIdentityIdStoreRootTreeLeaf,
        identityIdBuffers,
      );

      proof.setRootTreeProof(rootTreeProof);
      proof.setStoreTreeProof(storeTreeProof);
    } else {
      response.setIdentityIdsList(identityIdBuffers);
    }

    return new ResponseQuery({
      value: response.serializeBinary(),
    });
  }

  return identityIdsByPublicKeyHashesQueryHandler;
}

module.exports = identityIdsByPublicKeyHashesQueryHandlerFactory;
