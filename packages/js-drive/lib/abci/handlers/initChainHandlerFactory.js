const {
  tendermint: {
    abci: {
      ResponseInitChain,
    },
  },
} = require('@dashevo/abci/types');

const featureFlagsSystemIds = require('@dashevo/feature-flags-contract/lib/systemIds');
const featureFlagsDocuments = require('@dashevo/feature-flags-contract/schema/feature-flags-documents.json');

const dpnsSystemIds = require('@dashevo/dpns-contract/lib/systemIds');
const dpnsDocuments = require('@dashevo/dpns-contract/schema/dpns-contract-documents.json');

const masternodeRewardsSystemIds = require('@dashevo/masternode-reward-shares-contract/lib/systemIds');
const masternodeRewardsDocuments = require('@dashevo/masternode-reward-shares-contract/schema/masternode-reward-shares-documents.json');

/**
 * Init Chain ABCI handler
 *
 * @param {updateSimplifiedMasternodeList} updateSimplifiedMasternodeList
 * @param {number} initialCoreChainLockedHeight
 * @param {ValidatorSet} validatorSet
 * @param {createValidatorSetUpdate} createValidatorSetUpdate
 * @param {Object} systemContractOwnerIdPublicKeys
 * @param {registerSystemDataContract} registerSystemDataContract
 * @param {RootTree} rootTree
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
  systemContractOwnerIdPublicKeys,
  registerSystemDataContract,
  rootTree,
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

    contextLogger.debug('Registering system data contract: feature flags');
    contextLogger.trace({
      ownerId: featureFlagsSystemIds.ownerId,
      contractId: featureFlagsSystemIds.contractId,
      publicKey: systemContractOwnerIdPublicKeys.featureFlags,
    });

    // Registering feature flags data contract
    await registerSystemDataContract(
      featureFlagsSystemIds.ownerId,
      featureFlagsSystemIds.contractId,
      systemContractOwnerIdPublicKeys.featureFlags,
      featureFlagsDocuments,
    );

    contextLogger.debug('Registering system data contract: DPNS');
    contextLogger.trace({
      ownerId: dpnsSystemIds.ownerId,
      contractId: dpnsSystemIds.contractId,
      publicKey: systemContractOwnerIdPublicKeys.dpns,
    });

    // Registering DPNS data contract
    await registerSystemDataContract(
      dpnsSystemIds.ownerId,
      dpnsSystemIds.contractId,
      systemContractOwnerIdPublicKeys.dpns,
      dpnsDocuments,
    );

    contextLogger.debug('Registering system data contract: masternode rewards');
    contextLogger.trace({
      ownerId: masternodeRewardsSystemIds.ownerId,
      contractId: masternodeRewardsSystemIds.contractId,
      publicKey: systemContractOwnerIdPublicKeys.masternodeRewards,
    });

    // Registering masternode reward sharing data contract
    await registerSystemDataContract(
      masternodeRewardsSystemIds.ownerId,
      masternodeRewardsSystemIds.contractId,
      systemContractOwnerIdPublicKeys.masternodeRewards,
      masternodeRewardsDocuments,
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
    });
  }

  return initChainHandler;
}

module.exports = initChainHandlerFactory;
