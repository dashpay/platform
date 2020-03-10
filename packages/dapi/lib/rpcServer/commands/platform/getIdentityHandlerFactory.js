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

    const { result, error: jsonRpcError } = await rpcClient.request('abci_query', { path, data });

    if (jsonRpcError) {
      const error = new Error();
      Object.assign(error, jsonRpcError);

      throw error;
    }

    handleAbciResponse(result.response);

    const { response: { value: identityBase64 } } = result;

    return { identity: identityBase64 };
  }

  return getIdentityHandler;
}

module.exports = getIdentityHandlerFactory;
