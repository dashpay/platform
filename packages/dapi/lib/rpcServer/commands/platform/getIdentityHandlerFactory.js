/**
 *
 * @param {jaysonClient} rpcClient
 * @param {handleAbciResponse} handleAbciResponse
 * @param {Validator} validator
 * @returns {getIdentityHandler}
 */
function getIdentityHandlerFactory(rpcClient, handleAbciResponse, validator) {
  /**
   * @typedef getIdentityHandler
   * @param {Object} args
   */
  async function getIdentityHandler(args) {
    validator.validate(args);
    const { id } = args;

    const path = '/identity';

    const data = Buffer.from(id).toString('hex');

    const { result, error: errorMessage } = await rpcClient.request('abci_query', { path, data });

    if (errorMessage) {
      throw new Error(errorMessage);
    }

    handleAbciResponse(result.response);

    const { response: { value: identityBase64 } } = result;

    return { identity: identityBase64 };
  }

  return getIdentityHandler;
}

module.exports = getIdentityHandlerFactory;
