/* eslint-disable no-param-reassign */
const lodashSet = require('lodash.set');
const lodashGet = require('lodash.get');

const systemConfigs = require('./system');

const { NETWORK_TESTNET } = require('../src/constants');

module.exports = {
  '0.17.2': (configFile) => {
    Object.entries(configFile.configs).filter(([, config]) => config.network === NETWORK_TESTNET)
      .forEach((config) => {
        // Set DashPay contract ID and block height for testnet
        // Set seed nodes for testnet tenderdash
        lodashSet(config, 'platform.drive.tenderdash.p2p.seeds', systemConfigs.testnet.platform.drive.tenderdash.p2p.seeds);
        lodashSet(config, 'platform.drive.tenderdash.p2p.persistentPeers', []);
      });

    return configFile;
  },
  '0.17.3': (configFile) => {
    Object.entries(configFile.configs).filter(([, config]) => config.network === NETWORK_TESTNET)
      .forEach((config) => {
        // Set DashPay contract ID and block height for testnet
        lodashSet(config, 'platform.dashpay', systemConfigs.testnet.platform.dashpay);
      });

    return configFile;
  },
  '0.17.4': (configFile) => {
    Object.entries(configFile.configs).forEach(([name, config]) => {
      let baseConfig = systemConfigs.base;
      if (systemConfigs[name]) {
        baseConfig = systemConfigs[name];
      }

      const previousStdoutLogLevel = lodashGet(
        config,
        'platform.drive.abci.log.level',
      );

      // Set Drive's new logging variables
      lodashSet(config, 'platform.drive.abci.log', baseConfig.platform.drive.abci.log);

      // Keep previous log level for stdout
      if (previousStdoutLogLevel) {
        lodashSet(config, 'platform.drive.abci.log.stdout.level', previousStdoutLogLevel);
      }
    });
  },
  '0.18.0': (configFile) => {
    // Update docker images
    Object.entries(configFile.configs)
      .forEach(([, config]) => {
        lodashSet(config, 'core.sentinel', systemConfigs.base.core.sentinel);

        lodashSet(config, 'core.docker.image', systemConfigs.base.core.docker.image);

        lodashSet(
          config,
          'platform.drive.tenderdash.docker.image',
          systemConfigs.base.platform.drive.tenderdash.docker.image,
        );

        lodashSet(
          config,
          'platform.drive.abci.docker.image',
          systemConfigs.base.platform.drive.abci.docker.image,
        );

        lodashSet(
          config,
          'platform.dapi.api.docker.image',
          systemConfigs.base.platform.dapi.api.docker.image,
        );
      });

    return configFile;
  },
  '0.19.0': (configFile) => {
    // Add default group name if not present
    if (typeof configFile.defaultGroupName === 'undefined') {
      configFile.defaultGroupName = null;
    }

    Object.entries(configFile.configs)
      .forEach(([, config]) => {
        // Add groups
        if (typeof config.group === 'undefined') {
          config.group = null;
        }

        if (typeof config.compose !== 'undefined') {
          // Remove platform option for non platform configs
          if (!config.compose.file.includes('docker-compose.platform.yml')) {
            delete config.platform;
          }

          // Remove compose option
          delete config.compose;
        }

        if (typeof config.platform !== 'undefined') {
          // Add Tenderdash node ID
          config.platform.drive.tenderdash.nodeId = null;

          // Add build options for DAPI and Drive
          config.platform.drive.abci.docker.build = {
            path: null,
          };

          config.platform.dapi.api.docker.build = {
            path: null,
          };

          // Add consensus options
          config.platform.drive.tenderdash.consensus = systemConfigs.base
            .platform.drive.tenderdash.consensus;

          // Remove fallbacks
          if (typeof config.platform.drive.skipAssetLockConfirmationValidation !== 'undefined') {
            delete config.platform.drive.skipAssetLockConfirmationValidation;
          }

          if (typeof config.platform.drive.passFakeAssetLockProofForTests !== 'undefined') {
            delete config.platform.drive.passFakeAssetLockProofForTests;
          }

          if (!config.platform.featureFlags) {
            config.platform.featureFlags = systemConfigs.base.platform.featureFlags;
          }

          // Remove Insight API configuration
          if (config.platform.dapi.insight) {
            delete config.platform.dapi.insight;
          }
        }

        // Update image versions
        config.core.docker.image = systemConfigs.base.core.docker.image;
        config.platform.dapi.api.docker.image = systemConfigs.base.platform.dapi.api.docker.image;
        config.platform.drive.abci.docker.image = systemConfigs.base.platform
          .drive.abci.docker.image;
      });

    // Update testnet seeds, genesis and contracts
    configFile.configs.testnet.platform.drive.tenderdash.p2p.seeds = systemConfigs.testnet.platform
      .drive.tenderdash.p2p.seeds;
    configFile.configs.testnet.platform.drive.tenderdash.genesis = systemConfigs.testnet.platform
      .drive.tenderdash.genesis;

    configFile.configs.testnet.platform.dpns = systemConfigs.testnet.platform.dpns;
    configFile.configs.testnet.platform.dashpay = systemConfigs.testnet.platform.dashpay;
    configFile.configs.testnet.platform.featureFlags = systemConfigs.testnet.platform.featureFlags;

    // Replace local config to group template
    configFile.configs.local = systemConfigs.local;

    return configFile;
  },
  '0.19.1': (configFile) => {
    Object.entries(configFile.configs)
      .forEach(([, config]) => {
        // Update image version
        config.core.docker.image = systemConfigs.base.core.docker.image;
      });

    return configFile;
  },
  '0.19.2': (configFile) => {
    Object.entries(configFile.configs)
      .forEach(([, config]) => {
        // Update image version
        config.core.docker.image = systemConfigs.base.core.docker.image;
        config.core.sentinel.docker.image = systemConfigs.base.core.sentinel.docker.image;
      });

    return configFile;
  },
  '0.20.0': (configFile) => {
    Object.entries(configFile.configs)
      .forEach(([, config]) => {
        // Core debug
        if (typeof config.core.debug === 'undefined') {
          config.core.debug = 0;
        }

        if (config.platform) {
          // Set empty block interval back to 3
          if (config.platform.drive.tenderdash.consensus.createEmptyBlocks.createEmptyBlocksInterval === '10s') {
            // noinspection JSPrimitiveTypeWrapperUsage
            config.platform.drive.tenderdash.consensus.createEmptyBlocks.createEmptyBlocksInterval = '3m';
          }

          // Tenderdash logging levels
          if (typeof config.platform.drive.tenderdash.log === 'undefined') {
            config.platform.drive.tenderdash.log = systemConfigs.base.platform.drive.tenderdash.log;
          }

          // Remove validator set
          if (typeof config.platform.drive.tenderdash.validatorKey === 'undefined') {
            delete config.platform.drive.tenderdash.validatorKey;
          }

          // Update images
          config.platform.drive.tenderdash.docker.image = systemConfigs.base.platform
            .drive.tenderdash.docker.image;

          config.platform.drive.abci.docker.image = systemConfigs.base.platform
            .drive.abci.docker.image;

          config.platform.dapi.api.docker.image = systemConfigs.base.platform
            .dapi.api.docker.image;

          config.core.docker.image = systemConfigs.base.core.docker.image;

          config.core.sentinel.docker.image = systemConfigs.base.core.sentinel.docker.image;
        }
      });

    // Set validator set LLMQ Type
    configFile.configs.base.platform.drive.abci.validatorSet.llmqType = systemConfigs.base
      .platform.drive.abci.validatorSet.llmqType;

    configFile.configs.local.platform.drive.abci.validatorSet.llmqType = systemConfigs.local
      .platform.drive.abci.validatorSet.llmqType;

    Object.entries(configFile.configs)
      .filter(([, config]) => config.group === 'local' && config.platform)
      .forEach(([, config]) => {
        config.platform.drive.abci.validatorSet.llmqType = systemConfigs.local
          .platform.drive.abci.validatorSet.llmqType;
      });

    // Update testnet seeds, genesis and contracts
    configFile.configs.testnet.platform.drive.tenderdash.p2p.seeds = systemConfigs.testnet.platform
      .drive.tenderdash.p2p.seeds;
    configFile.configs.testnet.platform.drive.tenderdash.genesis = systemConfigs.testnet.platform
      .drive.tenderdash.genesis;

    configFile.configs.testnet.platform.dpns = systemConfigs.testnet.platform.dpns;
    configFile.configs.testnet.platform.dashpay = systemConfigs.testnet.platform.dashpay;
    configFile.configs.testnet.platform.featureFlags = systemConfigs.testnet.platform.featureFlags;

    return configFile;
  },
  '0.20.2': (configFile) => {
    // Update contracts
    configFile.configs.testnet.platform.drive.tenderdash.genesis = systemConfigs.testnet.platform
      .drive.tenderdash.genesis;
    configFile.configs.testnet.platform.dpns = systemConfigs.testnet.platform.dpns;
    configFile.configs.testnet.platform.dashpay = systemConfigs.testnet.platform.dashpay;
    configFile.configs.testnet.platform.featureFlags = systemConfigs.testnet.platform.featureFlags;

    return configFile;
  },
  '0.21.0': (configFile) => {
    Object.entries(configFile.configs)
      .forEach(([, config]) => {
        // Add median time to config
        config.core.miner.mediantime = systemConfigs.base.core.miner.mediantime;

        if (config.platform) {
          // Update images
          config.platform.drive.tenderdash.docker.image = systemConfigs.base.platform
            .drive.tenderdash.docker.image;

          config.platform.drive.abci.docker.image = systemConfigs.base.platform
            .drive.abci.docker.image;

          config.platform.dapi.api.docker.image = systemConfigs.base.platform
            .dapi.api.docker.image;
        }
      });

    // Update contracts
    configFile.configs.testnet.platform.drive.tenderdash.genesis = systemConfigs.testnet.platform
      .drive.tenderdash.genesis;
    configFile.configs.testnet.platform.dpns = systemConfigs.testnet.platform.dpns;
    configFile.configs.testnet.platform.dashpay = systemConfigs.testnet.platform.dashpay;
    configFile.configs.testnet.platform.featureFlags = systemConfigs.testnet.platform.featureFlags;

    return configFile;
  },
  '0.21.7': (configFile) => {
    Object.entries(configFile.configs)
      .forEach(([, config]) => {
        if (config.platform) {
          // Remove build setting
          delete config.platform.drive.abci.docker.build;

          delete config.platform.dapi.api.docker.build;

          config.platform.sourcePath = null;
        }
      });

    return configFile;
  },
  '0.22.0': (configFile) => {
    Object.entries(configFile.configs)
      .forEach(([, config]) => {
        config.docker = systemConfigs[config.group || 'base'].docker;

        // Update images
        config.core.docker.image = systemConfigs.base.core.docker.image;

        if (config.platform) {
          if (!config.platform.masternodeRewardShares) {
            config.platform.masternodeRewardShares = systemConfigs.base.platform
              .masternodeRewardShares;
          }

          config.platform.drive.tenderdash.docker.image = systemConfigs.base.platform
            .drive.tenderdash.docker.image;

          config.platform.drive.abci.docker.image = systemConfigs.base.platform
            .drive.abci.docker.image;

          config.platform.dapi.api.docker.image = systemConfigs.base.platform
            .dapi.api.docker.image;

          delete config.platform.drive.mongodb;
        }
      });

    // Update testnet contracts
    configFile.configs.testnet.platform.drive.tenderdash.genesis = systemConfigs.testnet.platform
      .drive.tenderdash.genesis;
    configFile.configs.testnet.platform.dpns = systemConfigs.testnet.platform.dpns;
    configFile.configs.testnet.platform.dashpay = systemConfigs.testnet.platform.dashpay;
    configFile.configs.testnet.platform.featureFlags = systemConfigs.testnet.platform.featureFlags;
    configFile.configs.testnet.platform.masternodeRewardShares = systemConfigs.testnet.platform
      .masternodeRewardShares;

    return configFile;
  },
  '0.22.2': (configFile) => {
    Object.entries(configFile.configs)
      .forEach(([, config]) => {
        config.core.docker.image = systemConfigs.base.core.docker.image;
      });

    return configFile;
  },
  '0.23.0': (configFile) => {
    Object.entries(configFile.configs)
      .forEach(([, config]) => {
        if (config.platform) {
          // Update images
          config.platform.dpns = systemConfigs.base.platform.dpns;
          config.platform.featureFlags = systemConfigs.base.platform.featureFlags;
          config.platform.dashpay = systemConfigs.base.platform.dashpay;
          config.platform.masternodeRewardShares = systemConfigs.base.platform
            .masternodeRewardShares;
        }
      });

    configFile.configs.testnet.platform.dpns = systemConfigs.testnet.platform.dpns;
    configFile.configs.testnet.platform.dashpay = systemConfigs.testnet.platform.dashpay;
    configFile.configs.testnet.platform.featureFlags = systemConfigs.testnet.platform.featureFlags;
    configFile.configs.testnet.platform.masternodeRewardShares = systemConfigs.testnet.platform
      .masternodeRewardShares;

    return configFile;
  },
};
