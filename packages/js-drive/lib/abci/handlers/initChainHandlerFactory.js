const {
  tendermint: {
    abci: {
      ResponseInitChain,
    },
  },
} = require('@dashevo/abci/types');

const { asValue } = require('awilix');

/**
 * Init Chain ABCI handler
 *
 * @param {updateSimplifiedMasternodeList} updateSimplifiedMasternodeList
 * @param {number} initialCoreChainLockedHeight
 * @param {ValidatorSet} validatorSet
 * @param {createValidatorSetUpdate} createValidatorSetUpdate
 * @param {BaseLogger} logger
 * @param {registerSystemDataContract} registerSystemDataContract
 * @param {registerTopLevelDomain} registerTopLevelDomain
 * @param {registerFeatureFlag} registerFeatureFlag
 * @param {RootTree} rootTree
 * @param {DocumentDatabaseManager} documentDatabaseManager
 * @param {DocumentDatabaseManager} previousDocumentDatabaseManager
 * @param {Identifier} dpnsContractId
 * @param {Identifier} dpnsOwnerId
 * @param {PublicKey} dpnsOwnerPublicKey
 * @param {Object} dpnsDocuments
 * @param {Identifier} featureFlagsContractId
 * @param {Identifier} featureFlagsOwnerId
 * @param {PublicKey} featureFlagsOwnerPublicKey
 * @param {Object} featureFlagsDocuments
 * @param {Identifier} masternodeRewardSharesContractId
 * @param {Identifier} masternodeRewardSharesOwnerId
 * @param {PublicKey} masternodeRewardSharesOwnerPublicKey
 * @param {Object} masternodeRewardSharesDocuments
 * @param {Identifier} dashpayContractId
 * @param {Identifier} dashpayOwnerId
 * @param {PublicKey} dashpayOwnerPublicKey
 * @param {Object} dashpayDocuments
 * @param {BlockExecutionStoreTransactions} blockExecutionStoreTransactions
 * @param {cloneToPreviousStoreTransactions} cloneToPreviousStoreTransactions
 * @param {AwilixContainer} container
 *
 * @return {initChainHandler}
 */
function initChainHandlerFactory(
  updateSimplifiedMasternodeList,
  initialCoreChainLockedHeight,
  validatorSet,
  createValidatorSetUpdate,
  logger,
  registerSystemDataContract,
  registerTopLevelDomain,
  registerFeatureFlag,
  rootTree,
  documentDatabaseManager,
  previousDocumentDatabaseManager,
  dpnsContractId,
  dpnsOwnerId,
  dpnsOwnerPublicKey,
  dpnsDocuments,
  featureFlagsContractId,
  featureFlagsOwnerId,
  featureFlagsOwnerPublicKey,
  featureFlagsDocuments,
  masternodeRewardSharesContractId,
  masternodeRewardSharesOwnerId,
  masternodeRewardSharesOwnerPublicKey,
  masternodeRewardSharesDocuments,
  dashpayContractId,
  dashpayOwnerId,
  dashpayOwnerPublicKey,
  dashpayDocuments,
  blockExecutionStoreTransactions,
  cloneToPreviousStoreTransactions,
  container,
) {
  /**
   * @typedef initChainHandler
   *
   * @param {abci.RequestInitChain} request
   * @return {Promise<abci.ResponseInitChain>}
   */
  async function initChainHandler(request) {
    const { time } = request;

    const genesisTime = new Date(
      time.seconds.toNumber() * 1000,
    );

    const contextLogger = logger.child({
      height: request.initialHeight.toString(),
      abciMethod: 'initChain',
    });

    contextLogger.debug('InitChain ABCI method requested');
    contextLogger.trace({ abciRequest: request });

    await blockExecutionStoreTransactions.start();

    const previousBlockExecutionStoreTransactions = await cloneToPreviousStoreTransactions(
      blockExecutionStoreTransactions,
    );

    container.register({
      previousBlockExecutionStoreTransactions: asValue(previousBlockExecutionStoreTransactions),
    });

    await blockExecutionStoreTransactions.commit();

    contextLogger.debug('Registering system data contract: feature flags');
    contextLogger.trace({
      ownerId: featureFlagsOwnerId,
      contractId: featureFlagsContractId,
      publicKey: featureFlagsOwnerPublicKey,
    });

    // Registering feature flags data contract
    const featureFlagContract = await registerSystemDataContract(
      featureFlagsOwnerId,
      featureFlagsContractId,
      featureFlagsOwnerPublicKey,
      featureFlagsDocuments,
    );

    await documentDatabaseManager.create(
      featureFlagContract, { isTransactional: false },
    );
    await previousDocumentDatabaseManager.create(
      featureFlagContract, { isTransactional: false },
    );

    await registerFeatureFlag(
      'fixCumulativeFeesBug',
      featureFlagContract,
      featureFlagsOwnerId,
      genesisTime,
    );

    contextLogger.debug('Registering system data contract: DPNS');
    contextLogger.trace({
      ownerId: dpnsOwnerId,
      contractId: dpnsContractId,
      publicKey: dpnsOwnerPublicKey,
    });

    // Registering DPNS data contract
    const dpnsContract = await registerSystemDataContract(
      dpnsOwnerId,
      dpnsContractId,
      dpnsOwnerPublicKey,
      dpnsDocuments,
    );

    await documentDatabaseManager.create(
      dpnsContract, { isTransactional: false },
    );
    await previousDocumentDatabaseManager.create(
      dpnsContract, { isTransactional: false },
    );

    await registerTopLevelDomain('dash', dpnsContract, dpnsOwnerId, genesisTime);

    contextLogger.debug('Registering system data contract: masternode rewards');
    contextLogger.trace({
      ownerId: masternodeRewardSharesOwnerId,
      contractId: masternodeRewardSharesContractId,
      publicKey: masternodeRewardSharesOwnerPublicKey,
    });

    // Registering masternode reward sharing data contract
    const masternodeRewardSharesContract = await registerSystemDataContract(
      masternodeRewardSharesOwnerId,
      masternodeRewardSharesContractId,
      masternodeRewardSharesOwnerPublicKey,
      masternodeRewardSharesDocuments,
    );

    await documentDatabaseManager.create(
      masternodeRewardSharesContract,
      { isTransactional: false },
    );
    await previousDocumentDatabaseManager.create(
      masternodeRewardSharesContract,
      { isTransactional: false },
    );

    contextLogger.debug('Registering system data contract: dashpay');
    contextLogger.trace({
      ownerId: dashpayOwnerId,
      contractId: dashpayContractId,
      publicKey: dashpayOwnerPublicKey,
    });

    // Registering masternode reward sharing data contract
    const dashpayContract = await registerSystemDataContract(
      dashpayOwnerId,
      dashpayContractId,
      dashpayOwnerPublicKey,
      dashpayDocuments,
    );

    await documentDatabaseManager.create(
      dashpayContract, { isTransactional: false },
    );
    await previousDocumentDatabaseManager.create(
      dashpayContract, { isTransactional: false },
    );

    await updateSimplifiedMasternodeList(initialCoreChainLockedHeight, {
      logger: contextLogger,
    });

    contextLogger.info(`Init ${request.chainId} chain on block #${request.initialHeight.toString()}`);

    await validatorSet.initialize(initialCoreChainLockedHeight);

    const { quorumHash } = validatorSet.getQuorum();

    const validatorSetUpdate = createValidatorSetUpdate(validatorSet);

    contextLogger.trace(validatorSetUpdate, `Validator set initialized with ${quorumHash} quorum`);

    const appHash = rootTree.getRootHash();

    return new ResponseInitChain({
      appHash,
      validatorSetUpdate,
      initialCoreHeight: initialCoreChainLockedHeight,
    });
  }

  return initChainHandler;
}

module.exports = initChainHandlerFactory;
