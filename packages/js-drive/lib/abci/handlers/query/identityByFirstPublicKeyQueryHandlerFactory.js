const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const NotFoundAbciError = require('../../errors/NotFoundAbciError');

/**
 *
 * @param {PublicKeyIdentityIdMapLevelDBRepository} publicKeyIdentityIdRepository
 * @param {IdentityLevelDBRepository} identityRepository
 * @return {identityByFirstPublicKeyQueryHandler}
 */
function identityByFirstPublicKeyQueryHandlerFactory(
  publicKeyIdentityIdRepository,
  identityRepository,
) {
  /**
   * @typedef identityByFirstPublicKeyQueryHandler
   * @param {Object} params
   * @param {string} params.publicKeyHash
   * @return {Promise<ResponseQuery>}
   */
  async function identityByFirstPublicKeyQueryHandler({ publicKeyHash }) {
    const identityId = await publicKeyIdentityIdRepository.fetch(publicKeyHash);

    if (!identityId) {
      throw new NotFoundAbciError('Identity not found');
    }

    const identity = await identityRepository.fetch(identityId);

    if (!identity) {
      throw new NotFoundAbciError('Identity not found');
    }

    return new ResponseQuery({
      value: identity.serialize(),
    });
  }

  return identityByFirstPublicKeyQueryHandler;
}

module.exports = identityByFirstPublicKeyQueryHandlerFactory;
