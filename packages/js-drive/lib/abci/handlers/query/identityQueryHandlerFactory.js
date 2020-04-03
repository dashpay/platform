const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const NotFoundAbciError = require('../../errors/NotFoundAbciError');

/**
 *
 * @param {IdentityLevelDBRepository} identityRepository
 * @return {identityQueryHandler}
 */
function identityQueryHandlerFactory(identityRepository) {
  /**
   * @typedef identityQueryHandler
   * @param {Object} params
   * @param {string} params.id
   * @return {Promise<ResponseQuery>}
   */
  async function identityQueryHandler({ id }) {
    const identity = await identityRepository.fetch(id);

    if (!identity) {
      throw new NotFoundAbciError('Identity not found');
    }

    return new ResponseQuery({
      value: identity.serialize(),
    });
  }

  return identityQueryHandler;
}

module.exports = identityQueryHandlerFactory;
