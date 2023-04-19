const { Listr } = require('listr2');

/**
 * @param {generateHDPrivateKeys} generateHDPrivateKeys
 * @return {configureTenderdashTask}
 */
function generateSystemDataContractKeysTaskFactory(generateHDPrivateKeys) {
  /**
   * @typedef {configureTenderdashTask}
   * @return {Listr}
   * @param config
   * @param network
   */
  function generateSystemDataContractKeysTask(config, network) {
    return new Listr([
      {
        task: async () => {
          const {
            derivedPrivateKeys: [
              dpnsDerivedMasterPrivateKey,
              dpnsDerivedSecondPrivateKey,
            ],
          } = await generateHDPrivateKeys(network, [0, 1]);

          const {
            derivedPrivateKeys: [
              featureFlagsDerivedMasterPrivateKey,
              featureFlagsDerivedSecondPrivateKey,
            ],
          } = await generateHDPrivateKeys(network, [0, 1]);

          const {
            derivedPrivateKeys: [
              dashpayDerivedMasterPrivateKey,
              dashpayDerivedSecondPrivateKey,
            ],
          } = await generateHDPrivateKeys(network, [0, 1]);

          const {
            derivedPrivateKeys: [
              masternodeRewardSharesDerivedMasterPrivateKey,
              masternodeRewardSharesDerivedSecondPrivateKey,
            ],
          } = await generateHDPrivateKeys(network, [0, 1]);

          config.set('platform.dpns.masterPublicKey', dpnsDerivedMasterPrivateKey.privateKey.toPublicKey().toString());
          config.set('platform.dpns.secondPublicKey', dpnsDerivedSecondPrivateKey.privateKey.toPublicKey().toString());

          config.set('platform.featureFlags.masterPublicKey', featureFlagsDerivedMasterPrivateKey.privateKey.toPublicKey().toString());
          config.set('platform.featureFlags.secondPublicKey', featureFlagsDerivedSecondPrivateKey.privateKey.toPublicKey().toString());

          config.set('platform.dashpay.masterPublicKey', dashpayDerivedMasterPrivateKey.privateKey.toPublicKey().toString());
          config.set('platform.dashpay.secondPublicKey', dashpayDerivedSecondPrivateKey.privateKey.toPublicKey().toString());

          config.set('platform.masternodeRewardShares.masterPublicKey',
            masternodeRewardSharesDerivedMasterPrivateKey.privateKey
              .toPublicKey().toString());
          config.set('platform.masternodeRewardShares.secondPublicKey',
            masternodeRewardSharesDerivedSecondPrivateKey.privateKey
              .toPublicKey().toString());
        },
      },
    ]);
  }

  return generateSystemDataContractKeysTask;
}

module.exports = generateSystemDataContractKeysTaskFactory;
