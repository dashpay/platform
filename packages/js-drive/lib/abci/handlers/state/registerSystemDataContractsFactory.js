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
) {
  /**
   * @typedef {registerSystemDataContracts}
   *
   * @param {BaseLogger} contextLogger
   * @param {{ seconds: Long }} genesisTime
   *
   * @return {Promise<void>}
   */
  async function registerSystemDataContracts(contextLogger, genesisTime) {
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
    );

    const genesisDate = new Date(
      genesisTime.seconds.toNumber() * 1000,
    );

    await registerTopLevelDomain('dash', dpnsContract, dpnsOwnerId, genesisDate);

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
    );
  }

  return registerSystemDataContracts;
}

module.exports = registerSystemDataContractsFactory;
