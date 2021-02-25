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
  '0.19.0-dev': (configFile) => {
    // Add default group name if not present
    if (typeof configFile.defaultGroupName === 'undefined') {
      configFile.defaultGroupName = null;
    }

    // Add groups to existing configs
    Object.entries(configFile.configs)
      .forEach(([, config]) => {
        if (typeof config.group === 'undefined') {
          config.group = null;
        }

        config.platform.drive.tenderdash.nodeId = null;
      });

    // Replace local config to group template
    configFile.configs.local = systemConfigs.local;

    return configFile;
  },
};
