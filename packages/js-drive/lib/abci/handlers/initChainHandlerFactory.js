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
 * @param {synchronizeMasternodeIdentities} synchronizeMasternodeIdentities
 * @param {BaseLogger} logger
 * @param {createInitialStateStructure} createInitialStateStructure
 * @param {registerSystemDataContracts} registerSystemDataContracts
 * @param {GroveDBStore} groveDBStore
 *
 * @return {initChainHandler}
 */
function initChainHandlerFactory(
  updateSimplifiedMasternodeList,
  initialCoreChainLockedHeight,
  validatorSet,
  createValidatorSetUpdate,
  synchronizeMasternodeIdentities,
  logger,
  createInitialStateStructure,
  registerSystemDataContracts,
  groveDBStore,
) {
  /**
   * @typedef initChainHandler
   *
   * @param {abci.RequestInitChain} request
   * @return {Promise<abci.ResponseInitChain>}
   */
  async function initChainHandler(request) {
    const { time } = request;

    const contextLogger = logger.child({
      height: request.initialHeight.toString(),
      abciMethod: 'initChain',
    });

    contextLogger.debug('InitChain ABCI method requested');
    contextLogger.trace({ abciRequest: request });

    await updateSimplifiedMasternodeList(
      initialCoreChainLockedHeight, {
        logger: contextLogger,
      },
    );

    // Create initial state

    await groveDBStore.startTransaction();

    await createInitialStateStructure();

    await registerSystemDataContracts(contextLogger, time);

    await synchronizeMasternodeIdentities(initialCoreChainLockedHeight);

    await groveDBStore.commitTransaction();

    const appHash = await groveDBStore.getRootHash();

    // Set initial validator set

    await validatorSet.initialize(initialCoreChainLockedHeight);

    const { quorumHash } = validatorSet.getQuorum();

    const validatorSetUpdate = createValidatorSetUpdate(validatorSet);

    contextLogger.trace(validatorSetUpdate, `Validator set initialized with ${quorumHash} quorum`);

    contextLogger.info(
      {
        chainId: request.chainId,
        appHash: appHash.toString('hex').toUpperCase(),
        initialHeight: request.initialHeight.toString(),
        initialCoreHeight: initialCoreChainLockedHeight,
      },
      `Init ${request.chainId} chain on block #${request.initialHeight.toString()} with app hash ${appHash.toString('hex').toUpperCase()}`,
    );

    return new ResponseInitChain({
      appHash,
      validatorSetUpdate,
      initialCoreHeight: initialCoreChainLockedHeight,
    });
  }

  return initChainHandler;
}

module.exports = initChainHandlerFactory;
