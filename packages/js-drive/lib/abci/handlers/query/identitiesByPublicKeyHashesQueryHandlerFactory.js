const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const cbor = require('cbor');

const {
  v0: {
    GetIdentitiesByPublicKeyHashesResponse,
    ResponseMetadata,
    StoreTreeProofs,
  },
} = require('@dashevo/dapi-grpc');

const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');

/**
 *
 * @param {PublicKeyToIdentityIdStoreRepository} previousPublicKeyToIdentityIdRepository
 * @param {IdentityStoreRepository} previousIdentityRepository
 * @param {number} maxIdentitiesPerRequest
 * @param {RootTree} previousRootTree
 * @param {IdentitiesStoreRootTreeLeaf} previousIdentitiesStoreRootTreeLeaf
 * @param {PublicKeyToIdentityIdStoreRootTreeLeaf} previousPublicKeyToIdentityIdStoreRootTreeLeaf
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
  previousPublicKeyToIdentityIdStoreRootTreeLeaf,
  createQueryResponse,
  blockExecutionContext,
  previousBlockExecutionContext,
) {
  /**
   * @typedef identitiesByPublicKeyHashesQueryHandler
   * @param {Object} params
   * @param {Object} data
   * @param {Buffer[]} data.publicKeyHashes
   * @param {RequestQuery} request
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

      response.setIdentitiesList(publicKeyHashes.map(() => cbor.encode([])));
      response.setMetadata(new ResponseMetadata());

      return new ResponseQuery({
        value: response.serializeBinary(),
      });
    }

    const response = createQueryResponse(GetIdentitiesByPublicKeyHashesResponse, request.prove);

    const identityIds = await Promise.all(
      publicKeyHashes.map((publicKeyHash) => (
        previousPublicKeyToIdentityIdRepository.fetch(publicKeyHash)
      )),
    );

    const notFoundIdentityPublicKeyHashes = [];
    const foundIdentityIds = [];

    for (let i = 0; i < identityIds.length; i++) {
      // If identity id was not found, we need to request non-inclusion proof by public key hash
      if (!identityIds[i]) {
        notFoundIdentityPublicKeyHashes.push(publicKeyHashes[i]);
      } else {
        // If identity was found, we need to request ordinary identity proof by id
        foundIdentityIds.push(identityIds[i]);
      }
    }

    if (request.prove) {
      const proof = response.getProof();
      const storeTreeProofs = new StoreTreeProofs();

      const identitiesStoreTreeProof = previousIdentitiesStoreRootTreeLeaf.getProof(
        foundIdentityIds,
      );

      const publicKeyStoreTreeProof = previousPublicKeyToIdentityIdStoreRootTreeLeaf.getProof(
        notFoundIdentityPublicKeyHashes,
      );

      const rootTreeProof = previousRootTree.getProof([
        previousIdentitiesStoreRootTreeLeaf,
        previousPublicKeyToIdentityIdStoreRootTreeLeaf,
      ]);

      storeTreeProofs.setIdentitiesProof(identitiesStoreTreeProof);
      storeTreeProofs.setPublicKeyHashesToIdentityIdsProof(publicKeyStoreTreeProof);

      proof.setRootTreeProof(rootTreeProof);
      proof.setStoreTreeProofs(storeTreeProofs);
    } else {
      const identityBuffers = await Promise.all(
        identityIds.map(async (serializedIds) => {
          if (!serializedIds) {
            return cbor.encode([]);
          }

          const ids = cbor.decode(serializedIds);

          const identities = await Promise.all(
            ids.map(async (id) => {
              const identity = await previousIdentityRepository.fetch(
                Identifier.from(id),
              );

              return identity.toBuffer();
            }),
          );

          return cbor.encode(identities);
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
