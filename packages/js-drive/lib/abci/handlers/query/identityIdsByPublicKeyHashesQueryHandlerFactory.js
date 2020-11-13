const cbor = require('cbor');

const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');

/**
 *
 * @param {PublicKeyToIdentityIdStoreRepository} publicKeyToIdentityIdRepository
 * @param {number} maxIdentitiesPerRequest
 * @param {RootTree} rootTree
 * @param {PublicKeyToIdentityIdStoreRootTreeLeaf} publicKeyToIdentityIdStoreRootTreeLeaf
 * @return {identityIdsByPublicKeyHashesQueryHandler}
 */
function identityIdsByPublicKeyHashesQueryHandlerFactory(
  publicKeyToIdentityIdRepository,
  maxIdentitiesPerRequest,
  rootTree,
  publicKeyToIdentityIdStoreRootTreeLeaf,
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

    const includeProof = request.prove === 'true';

    const identityIds = await Promise.all(
      publicKeyHashes.map(async (publicKeyHash) => {
        const identityId = await publicKeyToIdentityIdRepository.fetch(publicKeyHash);

        if (!identityId) {
          return Buffer.alloc(0);
        }

        return identityId;
      }),
    );

    const value = {
      data: identityIds,
    };

    if (includeProof) {
      value.proof = rootTree.getFullProof(
        publicKeyToIdentityIdStoreRootTreeLeaf,
        publicKeyHashes,
      );
    }

    return new ResponseQuery({
      value: await cbor.encodeAsync(value),
    });
  }

  return identityIdsByPublicKeyHashesQueryHandler;
}

module.exports = identityIdsByPublicKeyHashesQueryHandlerFactory;
