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

    const consensusLogger = logger.child({
      height: request.initialHeight.toString(),
      abciMethod: 'initChain',
    });

    consensusLogger.debug('InitChain ABCI method requested');
    consensusLogger.trace({ abciRequest: request });

    await updateSimplifiedMasternodeList(
      initialCoreChainLockedHeight, {
        logger: consensusLogger,
      },
    );

    // Create initial state

    await groveDBStore.startTransaction();

    await createInitialStateStructure();

    await registerSystemDataContracts(consensusLogger, time);

    const synchronizeMasternodeIdentitiesResult = await synchronizeMasternodeIdentities(
      initialCoreChainLockedHeight,
    );

    const {
      createdEntities, updatedEntities, removedEntities, fromHeight, toHeight,
    } = synchronizeMasternodeIdentitiesResult;

    consensusLogger.info(
      `Masternode identities are synced for heights from ${fromHeight} to ${toHeight}: ${createdEntities.length} created, ${updatedEntities.length} updated, ${removedEntities.length} removed`,
    );

    consensusLogger.trace(
      {
        createdEntities: createdEntities.map((item) => item.toJSON()),
        updatedEntities: updatedEntities.map((item) => item.toJSON()),
        removedEntities: removedEntities.map((item) => item.toJSON()),
      },
      'Synchronized masternode identities',
    );

    await groveDBStore.commitTransaction();

    const appHash = await groveDBStore.getRootHash();

    // Set initial validator set

    await validatorSet.initialize(initialCoreChainLockedHeight);

    const { quorumHash } = validatorSet.getQuorum();

    const validatorSetUpdate = createValidatorSetUpdate(validatorSet);

    consensusLogger.trace(validatorSetUpdate, `Validator set initialized with ${quorumHash} quorum`);

    consensusLogger.info(
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
