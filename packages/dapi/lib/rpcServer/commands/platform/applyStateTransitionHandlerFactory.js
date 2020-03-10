/**
 *
 * @param {jaysonClient} rpcClient
 * @param {handleAbciResponse} handleAbciResponse
 * @param {Validator} validator
 * @returns {applyStateTransitionHandler}
 */
function applyStateTransitionHandlerFactory(rpcClient, handleAbciResponse, validator) {
  /**
   * @typedef applyStateTransitionHandler
   * @param {Object} args
   * @param {string} args.stateTransition
   */
  async function applyStateTransitionHandler(args) {
    validator.validate(args);
    const { stateTransition: tx } = args;

    const { result, error: jsonRpcError } = await rpcClient.request('broadcast_tx_commit', { tx });

    if (jsonRpcError) {
      const error = new Error();
      Object.assign(error, jsonRpcError);

      throw error;
    }

    const { check_tx: checkTx, deliver_tx: deliverTx } = result;

    handleAbciResponse(checkTx);

    handleAbciResponse(deliverTx);

    return true;
  }

  return applyStateTransitionHandler;
}

module.exports = applyStateTransitionHandlerFactory;
