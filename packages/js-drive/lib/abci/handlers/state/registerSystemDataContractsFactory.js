/**
 *
 * @param {registerSystemDataContract} registerSystemDataContract
 * @param {registerTopLevelDomain} registerTopLevelDomain
 * @param {Identifier} dpnsContractId
 * @param {Identifier} dpnsOwnerId
 * @param {PublicKey} dpnsOwnerMasterPublicKey
 * @param {PublicKey} dpnsOwnerSecondPublicKey
 * @param {Object} dpnsDocuments
 * @param {Identifier} featureFlagsContractId
 * @param {Identifier} featureFlagsOwnerId
 * @param {PublicKey} featureFlagsOwnerMasterPublicKey
 * @param {PublicKey} featureFlagsOwnerSecondPublicKey
 * @param {Object} featureFlagsDocuments
 * @param {Identifier} masternodeRewardSharesContractId
 * @param {Identifier} masternodeRewardSharesOwnerId
 * @param {PublicKey} masternodeRewardSharesOwnerMasterPublicKey
 * @param {PublicKey} masternodeRewardSharesOwnerSecondPublicKey
 * @param {Object} masternodeRewardSharesDocuments
 * @param {Identifier} dashpayContractId
 * @param {Identifier} dashpayOwnerId
 * @param {PublicKey} dashpayOwnerMasterPublicKey
 * @param {PublicKey} dashpayOwnerSecondPublicKey
 * @param {Object} dashpayDocuments
 * @param {Identifier} withdrawalsContractId
 * @param {Identifier} withdrawalsOwnerId
 * @param {PublicKey} withdrawalsOwnerMasterPublicKey
 * @param {PublicKey} withdrawalsOwnerSecondPublicKey
 * @param {Object} withdrawalsDocuments
 *
 * @return {registerSystemDataContracts}
 */
function registerSystemDataContractsFactory(
  registerSystemDataContract,
  registerTopLevelDomain,
  dpnsContractId,
  dpnsOwnerId,
  dpnsOwnerMasterPublicKey,
  dpnsOwnerSecondPublicKey,
  dpnsDocuments,
  featureFlagsContractId,
  featureFlagsOwnerId,
  featureFlagsOwnerMasterPublicKey,
  featureFlagsOwnerSecondPublicKey,
  featureFlagsDocuments,
  masternodeRewardSharesContractId,
  masternodeRewardSharesOwnerId,
  masternodeRewardSharesOwnerMasterPublicKey,
  masternodeRewardSharesOwnerSecondPublicKey,
  masternodeRewardSharesDocuments,
  dashpayContractId,
  dashpayOwnerId,
  dashpayOwnerMasterPublicKey,
  dashpayOwnerSecondPublicKey,
  dashpayDocuments,
  withdrawalsContractId,
  withdrawalsOwnerId,
  withdrawalsOwnerMasterPublicKey,
  withdrawalsOwnerSecondPublicKey,
  withdrawalsDocuments,
) {
  /**
   * @typedef {registerSystemDataContracts}
   *
   * @param {BaseLogger} contextLogger
   * @param {BlockInfo} blockInfo
   *
   * @return {Promise<void>}
   */
  async function registerSystemDataContracts(contextLogger, blockInfo) {
    contextLogger.debug('Registering Feature Flags data contract');
    contextLogger.trace({
      ownerId: featureFlagsOwnerId,
      contractId: featureFlagsContractId,
      masterPublicKey: featureFlagsOwnerMasterPublicKey,
      secondPublicKey: featureFlagsOwnerSecondPublicKey,
    });

    // Registering feature flags data contract
    await registerSystemDataContract(
      featureFlagsOwnerId,
      featureFlagsContractId,
      featureFlagsOwnerMasterPublicKey,
      featureFlagsOwnerSecondPublicKey,
      featureFlagsDocuments,
      blockInfo,
    );

    contextLogger.debug('Registering DPNS data contract');
    contextLogger.trace({
      ownerId: dpnsOwnerId,
      contractId: dpnsContractId,
      masterPublicKey: dpnsOwnerMasterPublicKey,
      secondPublicKey: dpnsOwnerSecondPublicKey,
    });

    // Registering DPNS data contract
    const dpnsContract = await registerSystemDataContract(
      dpnsOwnerId,
      dpnsContractId,
      dpnsOwnerMasterPublicKey,
      dpnsOwnerSecondPublicKey,
      dpnsDocuments,
      blockInfo,
    );

    await registerTopLevelDomain('dash', dpnsContract, dpnsOwnerId, blockInfo);

    contextLogger.debug('Registering Masternode Rewards data contract');
    contextLogger.trace({
      ownerId: masternodeRewardSharesOwnerId,
      contractId: masternodeRewardSharesContractId,
      masterPublicKey: masternodeRewardSharesOwnerMasterPublicKey,
      secondPublicKey: masternodeRewardSharesOwnerSecondPublicKey,
    });

    // Registering masternode reward sharing data contract
    await registerSystemDataContract(
      masternodeRewardSharesOwnerId,
      masternodeRewardSharesContractId,
      masternodeRewardSharesOwnerMasterPublicKey,
      masternodeRewardSharesOwnerSecondPublicKey,
      masternodeRewardSharesDocuments,
      blockInfo,
    );

    contextLogger.debug('Registering Dashpay data contract');
    contextLogger.trace({
      ownerId: dashpayOwnerId,
      contractId: dashpayContractId,
      masterPublicKey: dashpayOwnerMasterPublicKey,
      secondPublicKey: dashpayOwnerSecondPublicKey,
    });

    // Registering masternode reward sharing data contract
    await registerSystemDataContract(
      dashpayOwnerId,
      dashpayContractId,
      dashpayOwnerMasterPublicKey,
      dashpayOwnerSecondPublicKey,
      dashpayDocuments,
      blockInfo,
    );

    contextLogger.debug('Registering withdrawals data contract');
    contextLogger.trace({
      ownerId: withdrawalsOwnerId,
      contractId: withdrawalsContractId,
      masterPublicKey: withdrawalsOwnerMasterPublicKey,
      secondPublicKey: withdrawalsOwnerSecondPublicKey,
    });

    // Registering withdrawals data contract
    await registerSystemDataContract(
      withdrawalsOwnerId,
      withdrawalsContractId,
      withdrawalsOwnerMasterPublicKey,
      withdrawalsOwnerSecondPublicKey,
      withdrawalsDocuments,
      blockInfo,
    );
  }

  return registerSystemDataContracts;
}

module.exports = registerSystemDataContractsFactory;
