/**
 *
 * @param {registerSystemDataContract} registerSystemDataContract
 * @param {registerTopLevelDomain} registerTopLevelDomain
 * @param {Identifier} dpnsContractId
 * @param {Identifier} dpnsOwnerId
 * @param {PublicKey} dpnsOwnerMasterPublicKey
 * @param {PublicKey} dpnsOwnerHighPublicKey
 * @param {Object} dpnsDocuments
 * @param {Identifier} featureFlagsContractId
 * @param {Identifier} featureFlagsOwnerId
 * @param {PublicKey} featureFlagsOwnerMasterPublicKey
 * @param {PublicKey} featureFlagsOwnerHighPublicKey
 * @param {Object} featureFlagsDocuments
 * @param {Identifier} masternodeRewardSharesContractId
 * @param {Identifier} masternodeRewardSharesOwnerId
 * @param {PublicKey} masternodeRewardSharesOwnerMasterPublicKey
 * @param {PublicKey} masternodeRewardSharesOwnerHighPublicKey
 * @param {Object} masternodeRewardSharesDocuments
 * @param {Identifier} dashpayContractId
 * @param {Identifier} dashpayOwnerId
 * @param {PublicKey} dashpayOwnerMasterPublicKey
 * @param {PublicKey} dashpayOwnerHighPublicKey
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
  dpnsOwnerHighPublicKey,
  dpnsDocuments,
  featureFlagsContractId,
  featureFlagsOwnerId,
  featureFlagsOwnerMasterPublicKey,
  featureFlagsOwnerHighPublicKey,
  featureFlagsDocuments,
  masternodeRewardSharesContractId,
  masternodeRewardSharesOwnerId,
  masternodeRewardSharesOwnerMasterPublicKey,
  masternodeRewardSharesOwnerHighPublicKey,
  masternodeRewardSharesDocuments,
  dashpayContractId,
  dashpayOwnerId,
  dashpayOwnerMasterPublicKey,
  dashpayOwnerHighPublicKey,
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
      highPublicKey: featureFlagsOwnerHighPublicKey,
    });

    // Registering feature flags data contract
    await registerSystemDataContract(
      featureFlagsOwnerId,
      featureFlagsContractId,
      featureFlagsOwnerMasterPublicKey,
      featureFlagsOwnerHighPublicKey,
      featureFlagsDocuments,
    );

    contextLogger.debug('Registering DPNS data contract');
    contextLogger.trace({
      ownerId: dpnsOwnerId,
      contractId: dpnsContractId,
      masterPublicKey: dpnsOwnerMasterPublicKey,
      highPublicKey: dpnsOwnerHighPublicKey,
    });

    // Registering DPNS data contract
    const dpnsContract = await registerSystemDataContract(
      dpnsOwnerId,
      dpnsContractId,
      dpnsOwnerMasterPublicKey,
      dpnsOwnerHighPublicKey,
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
      highPublicKey: masternodeRewardSharesOwnerHighPublicKey,
    });

    // Registering masternode reward sharing data contract
    await registerSystemDataContract(
      masternodeRewardSharesOwnerId,
      masternodeRewardSharesContractId,
      masternodeRewardSharesOwnerMasterPublicKey,
      masternodeRewardSharesOwnerHighPublicKey,
      masternodeRewardSharesDocuments,
    );

    contextLogger.debug('Registering Dashpay data contract');
    contextLogger.trace({
      ownerId: dashpayOwnerId,
      contractId: dashpayContractId,
      masterPublicKey: dashpayOwnerMasterPublicKey,
      highPublicKey: dashpayOwnerHighPublicKey,
    });

    // Registering masternode reward sharing data contract
    await registerSystemDataContract(
      dashpayOwnerId,
      dashpayContractId,
      dashpayOwnerMasterPublicKey,
      dashpayOwnerHighPublicKey,
      dashpayDocuments,
    );
  }

  return registerSystemDataContracts;
}

module.exports = registerSystemDataContractsFactory;
