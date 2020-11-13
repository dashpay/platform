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
 * @param {IdentityStoreRepository} identityRepository
 * @param {number} maxIdentitiesPerRequest
 * @param {RootTree} rootTree
 * @param {IdentitiesStoreRootTreeLeaf} identitiesStoreRootTreeLeaf
 * @return {identitiesByPublicKeyHashesQueryHandler}
 */
function identitiesByPublicKeyHashesQueryHandlerFactory(
  publicKeyToIdentityIdRepository,
  identityRepository,
  maxIdentitiesPerRequest,
  rootTree,
  identitiesStoreRootTreeLeaf,
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

    const identityIds = [];

    const identities = await Promise.all(
      publicKeyHashes.map(async (publicKeyHash) => {
        const identityId = await publicKeyToIdentityIdRepository.fetch(publicKeyHash);

        if (!identityId) {
          return Buffer.alloc(0);
        }

        identityIds.push(identityId);

        const identity = await identityRepository.fetch(identityId);

        return identity.toBuffer();
      }),
    );

    const value = {
      data: identities,
    };

    const includeProof = request.prove === 'true';

    if (includeProof) {
      value.proof = rootTree.getFullProof(identitiesStoreRootTreeLeaf, identityIds);
    }

    return new ResponseQuery({
      value: await cbor.encodeAsync(value),
    });
  }

  return identitiesByPublicKeyHashesQueryHandler;
}

module.exports = identitiesByPublicKeyHashesQueryHandlerFactory;
