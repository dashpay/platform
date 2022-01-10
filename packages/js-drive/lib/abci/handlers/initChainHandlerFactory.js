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
 * @param {createInitialStateStructure} createInitialStateStructure
 * @param {groveDBStore} groveDBStore
 *
 * @return {initChainHandler}
 */
function initChainHandlerFactory(
  updateSimplifiedMasternodeList,
  initialCoreChainLockedHeight,
  validatorSet,
  createValidatorSetUpdate,
  logger,
  createInitialStateStructure,
  groveDBStore,
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

    await createInitialStateStructure();

    contextLogger.info(`Init ${request.chainId} chain on block #${request.initialHeight.toString()}`);

    await validatorSet.initialize(initialCoreChainLockedHeight);

    const { quorumHash } = validatorSet.getQuorum();

    const validatorSetUpdate = createValidatorSetUpdate(validatorSet);

    contextLogger.trace(validatorSetUpdate, `Validator set initialized with ${quorumHash} quorum`);

    const appHash = groveDBStore.getRootHash();

    return new ResponseInitChain({
      validatorSetUpdate,
      appHash,
      initialCoreHeight: initialCoreChainLockedHeight,
    });
  }

  return initChainHandler;
}

module.exports = initChainHandlerFactory;
