const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const {
  v0: {
    GetIdentityResponse,
  },
} = require('@dashevo/dapi-grpc');

const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const IdentifierError = require('@dashevo/dpp/lib/identifier/errors/IdentifierError');

const NotFoundAbciError = require('../../errors/NotFoundAbciError');
const UnimplementedAbciError = require('../../errors/UnimplementedAbciError');
const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');

/**
 *
 * @param {IdentityStoreRepository} signedIdentityRepository
 * @param {createQueryResponse} createQueryResponse
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContextStack} blockExecutionContextStack
 * @return {identityQueryHandler}
 */
function identityQueryHandlerFactory(
  signedIdentityRepository,
  createQueryResponse,
  blockExecutionContext,
  blockExecutionContextStack,
) {
  /**
   * @typedef identityQueryHandler
   * @param {Object} params
   * @param {Object} options
   * @param {Buffer} options.id
   * @param {RequestQuery} request
   * @return {Promise<ResponseQuery>}
   */
  async function identityQueryHandler(params, { id }, request) {
    // There is no signed state (current committed block height less than 3)
    if (!blockExecutionContextStack.getLast()) {
      throw new NotFoundAbciError('Identity not found');
    }

    let identifier;
    try {
      identifier = new Identifier(id);
    } catch (e) {
      if (e instanceof IdentifierError) {
        throw new InvalidArgumentAbciError('id must be a valid identifier (32 bytes long)');
      }

      throw e;
    }

    if (request.prove) {
      throw new UnimplementedAbciError('Proofs are not implemented yet');
    }

    const response = createQueryResponse(GetIdentityResponse, request.prove);

    const identity = await signedIdentityRepository.fetch(identifier);

    if (!identity) {
      throw new NotFoundAbciError('Identity not found');
    }

    response.setIdentity(identity.toBuffer());

    return new ResponseQuery({
      value: response.serializeBinary(),
    });
  }

  return identityQueryHandler;
}

module.exports = identityQueryHandlerFactory;
