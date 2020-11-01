const {
  abci: {
    ResponseEndBlock,
  },
} = require('abci/types');

const NoDPNSContractFoundError = require('./errors/NoDPNSContractFoundError');

/**
 * Begin block ABCI handler
 *
 * @param {BlockExecutionDBTransactions} blockExecutionDBTransactions
 * @param {DataContractLevelDBRepository} dataContractRepository
 * @param {number|undefined} dpnsContractBlockHeight
 * @param {Identifier|undefined} dpnsContractId
 * @param {BaseLogger} logger
 *
 * @return {endBlockHandler}
 */
function endBlockHandlerFactory(
  blockExecutionDBTransactions,
  dataContractRepository,
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
      const transaction = blockExecutionDBTransactions.getTransaction('dataContract');

      const contract = await dataContractRepository.fetch(dpnsContractId, transaction);

      if (contract === null) {
        throw new NoDPNSContractFoundError(dpnsContractId, dpnsContractBlockHeight);
      }
    }

    return new ResponseEndBlock();
  }

  return endBlockHandler;
}

module.exports = endBlockHandlerFactory;
