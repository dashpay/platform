const {
  tendermint: {
    abci: {
      ResponseInitChain,
    },
  },
} = require('@dashevo/abci/types');

/**
 * Init Chain ABCI handler
 *
 * @param {updateSimplifiedMasternodeList} updateSimplifiedMasternodeList
 * @param {number} initialCoreChainLockedHeight
 * @param {ValidatorSet} validatorSet
 * @param {createValidatorSetUpdate} createValidatorSetUpdate
 * @param {BaseLogger} logger
 *
 * @return {initChainHandler}
 */
function initChainHandlerFactory(
  updateSimplifiedMasternodeList,
  initialCoreChainLockedHeight,
  validatorSet,
  createValidatorSetUpdate,
  logger,
) {
  /**
   * @typedef initChainHandler
   *
   * @param {abci.RequestInitChain} request
   * @return {Promise<abci.ResponseInitChain>}
   */
  async function initChainHandler(request) {
    const contextLogger = logger.child({
      height: request.initialHeight.toString(),
      abciMethod: 'initChain',
    });

    contextLogger.debug('InitChain ABCI method requested');
    contextLogger.trace({ abciRequest: request });

    await updateSimplifiedMasternodeList(initialCoreChainLockedHeight, {
      logger: contextLogger,
    });

    contextLogger.info(`Init ${request.chainId} chain on block #${request.initialHeight.toString()}`);

    await validatorSet.initialize(initialCoreChainLockedHeight);

    const { quorumHash } = validatorSet.getQuorum();

    contextLogger.trace(`Initial validator set selected: quorumHash ${quorumHash}`);

    return new ResponseInitChain({
      validatorSetUpdate: createValidatorSetUpdate(validatorSet),
    });
  }

  return initChainHandler;
}

module.exports = initChainHandlerFactory;
