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
    ResponseMetadata,
    StoreTreeProofs,
  },
} = require('@dashevo/dapi-grpc');

const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');

/**
 *
 * @param {PublicKeyToIdentityIdStoreRepository} signedPublicKeyToIdentityIdRepository
 * @param {number} maxIdentitiesPerRequest
 * @param {createQueryResponse} createQueryResponse
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContext} blockExecutionContextStack
 * @return {identityIdsByPublicKeyHashesQueryHandler}
 */
function identityIdsByPublicKeyHashesQueryHandlerFactory(
  signedPublicKeyToIdentityIdRepository,
  maxIdentitiesPerRequest,
  createQueryResponse,
  blockExecutionContext,
  blockExecutionContextStack,
) {
  /**
   * @typedef identityIdsByPublicKeyHashesQueryHandler
   * @param {Object} params
   * @param {Object} data
   * @param {Buffer[]} data.publicKeyHashes
   * @param {RequestQuery} request
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

    // There is no signed state (current committed block height less than 3)
    if (!blockExecutionContextStack.getLast()) {
      const response = new GetIdentityIdsByPublicKeyHashesResponse();

      response.setIdentityIdsList(publicKeyHashes.map(() => Buffer.alloc(0)));

      response.setMetadata(new ResponseMetadata());

      return new ResponseQuery({
        value: response.serializeBinary(),
      });
    }

    const response = createQueryResponse(GetIdentityIdsByPublicKeyHashesResponse, request.prove);

    const identityIds = await Promise.all(
      publicKeyHashes.map(async (publicKeyHash) => (
        previousPublicKeyToIdentityIdRepository.fetch(publicKeyHash)
      )),
    );

    if (request.prove) {
      const proof = response.getProof();
      const storeTreeProofs = new StoreTreeProofs();

      const {
        rootTreeProof,
        storeTreeProof,
      } = previousRootTree.getFullProofForOneLeaf(
        previousPublicKeyToIdentityIdStoreRootTreeLeaf,
        publicKeyHashes,
      );

      storeTreeProofs.setPublicKeyHashesToIdentityIdsProof(storeTreeProof);

      proof.setRootTreeProof(rootTreeProof);
      proof.setStoreTreeProofs(storeTreeProofs);
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
