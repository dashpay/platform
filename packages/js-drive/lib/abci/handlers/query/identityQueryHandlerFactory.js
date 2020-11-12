const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const cbor = require('cbor');

const NotFoundAbciError = require('../../errors/NotFoundAbciError');

/**
 *
 * @param {IdentityStoreRepository} identityRepository
 * @param {RootTree} rootTree
 * @param {IdentitiesStoreRootTreeLeaf} identitiesStoreRootTreeLeaf
 * @return {identityQueryHandler}
 */
function identityQueryHandlerFactory(
  identityRepository,
  rootTree,
  identitiesStoreRootTreeLeaf,
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
    const includeProof = request.prove === 'true';

    const identity = await identityRepository.fetch(id);

    if (!identity) {
      throw new NotFoundAbciError('Identity not found');
    }

    const identityBuffer = identity.toBuffer();

    const value = {
      data: identityBuffer,
    };

    if (includeProof) {
      value.proof = rootTree.getFullProof(identitiesStoreRootTreeLeaf, [id]);
    }

    return new ResponseQuery({
      value: cbor.encode(value),
    });
  }

  return identityQueryHandler;
}

module.exports = identityQueryHandlerFactory;
