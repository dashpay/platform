/**
 *
 * @param {registerSystemDataContract} registerSystemDataContract
 * @param {registerTopLevelDomain} registerTopLevelDomain
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
 *
 * @return {registerSystemDataContracts}
 */
function registerSystemDataContractsFactory(
  registerSystemDataContract,
  registerTopLevelDomain,
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
      publicKey: featureFlagsOwnerPublicKey,
    });

    // Registering feature flags data contract
    await registerSystemDataContract(
      featureFlagsOwnerId,
      featureFlagsContractId,
      featureFlagsOwnerPublicKey,
      featureFlagsDocuments,
    );

    contextLogger.debug('Registering DPNS data contract');
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

    const genesisDate = new Date(
      genesisTime.seconds.toNumber() * 1000,
    );

    await registerTopLevelDomain('dash', dpnsContract, dpnsOwnerId, genesisDate);

    contextLogger.debug('Registering Masternode Rewards data contract');
    contextLogger.trace({
      ownerId: masternodeRewardSharesOwnerId,
      contractId: masternodeRewardSharesContractId,
      publicKey: masternodeRewardSharesOwnerPublicKey,
    });

    // Registering masternode reward sharing data contract
    await registerSystemDataContract(
      masternodeRewardSharesOwnerId,
      masternodeRewardSharesContractId,
      masternodeRewardSharesOwnerPublicKey,
      masternodeRewardSharesDocuments,
    );

    contextLogger.debug('Registering Dashpay data contract');
    contextLogger.trace({
      ownerId: dashpayOwnerId,
      contractId: dashpayContractId,
      publicKey: dashpayOwnerPublicKey,
    });

    // Registering masternode reward sharing data contract
    await registerSystemDataContract(
      dashpayOwnerId,
      dashpayContractId,
      dashpayOwnerPublicKey,
      dashpayDocuments,
    );
  }

  return registerSystemDataContracts;
}

module.exports = registerSystemDataContractsFactory;
