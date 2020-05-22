const bs58 = require('bs58');

const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const NotFoundAbciError = require('../../errors/NotFoundAbciError');

/**
 *
 * @param {PublicKeyIdentityIdMapLevelDBRepository} publicKeyIdentityIdRepository
 * @return {identityIdByFirstPublicKeyQueryHandler}
 */
function identityIdByFirstPublicKeyQueryHandlerFactory(publicKeyIdentityIdRepository) {
  /**
   * @typedef identityIdByFirstPublicKeyQueryHandler
   * @param {Object} params
   * @param {string} params.publicKeyHash
   * @return {Promise<ResponseQuery>}
   */
  async function identityIdByFirstPublicKeyQueryHandler({ publicKeyHash }) {
    const identityId = await publicKeyIdentityIdRepository.fetch(publicKeyHash);

    if (!identityId) {
      throw new NotFoundAbciError('Identity not found');
    }

    return new ResponseQuery({
      value: bs58.decode(identityId),
    });
  }

  return identityIdByFirstPublicKeyQueryHandler;
}

module.exports = identityIdByFirstPublicKeyQueryHandlerFactory;
