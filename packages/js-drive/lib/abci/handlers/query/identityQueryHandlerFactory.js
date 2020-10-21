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
   * @param {Object} options
   * @param {Buffer} options.id
   * @return {Promise<ResponseQuery>}
   */
  async function identityQueryHandler(params, { id }) {
    const identity = await identityRepository.fetch(id);

    if (!identity) {
      throw new NotFoundAbciError('Identity not found');
    }

    return new ResponseQuery({
      value: identity.toBuffer(),
    });
  }

  return identityQueryHandler;
}

module.exports = identityQueryHandlerFactory;
