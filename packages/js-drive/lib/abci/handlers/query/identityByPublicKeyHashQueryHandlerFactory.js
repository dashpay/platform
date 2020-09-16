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
 * @return {identityByPublicKeyHashQueryHandler}
 */
function identityByPublicKeyHashQueryHandlerFactory(
  publicKeyIdentityIdRepository,
  identityRepository,
) {
  /**
   * @typedef identityByPublicKeyHashQueryHandler
   * @param {Object} params
   * @param {string} params.publicKeyHash
   * @return {Promise<ResponseQuery>}
   */
  async function identityByPublicKeyHashQueryHandler({ publicKeyHash }) {
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

  return identityByPublicKeyHashQueryHandler;
}

module.exports = identityByPublicKeyHashQueryHandlerFactory;
