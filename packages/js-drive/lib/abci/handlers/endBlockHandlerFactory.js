const {
  abci: {
    ResponseEndBlock,
  },
} = require('abci/types');

const NoDPNSContractFoundError = require('./errors/NoDPNSContractFoundError');

/**
 * Begin block ABCI handler
 *
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {number|undefined} dpnsContractBlockHeight
 * @param {Identifier|undefined} dpnsContractId
 * @param {BaseLogger} logger
 *
 * @return {endBlockHandler}
 */
function endBlockHandlerFactory(
  blockExecutionContext,
  dpnsContractBlockHeight,
  dpnsContractId,
  logger,
) {
  /**
   * @typedef endBlockHandler
   *
   * @param {abci.RequestBeginBlock} request
   * @return {Promise<abci.ResponseBeginBlock>}
   */
  async function endBlockHandler({ height }) {
    logger.info(`Block end #${height}`);

    if (dpnsContractId && height === dpnsContractBlockHeight) {
      if (!blockExecutionContext.hasDataContract(dpnsContractId)) {
        throw new NoDPNSContractFoundError(dpnsContractId, dpnsContractBlockHeight);
      }
    }

    return new ResponseEndBlock();
  }

  return endBlockHandler;
}

module.exports = endBlockHandlerFactory;
