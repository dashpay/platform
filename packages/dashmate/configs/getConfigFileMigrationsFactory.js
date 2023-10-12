/* eslint-disable no-param-reassign */

const { NETWORK_LOCAL, NETWORK_TESTNET, NETWORK_MAINNET } = require('../src/constants');

/**
 * @param {HomeDir} homeDir
 * @param {DefaultConfigs} defaultConfigs
 * @returns {getConfigFileMigrations}
 */
function getConfigFileMigrationsFactory(homeDir, defaultConfigs) {
  /**
   * @typedef {function} getConfigFileMigrations
   * @returns {Object}
   */
  function getConfigFileMigrations() {
    const base = defaultConfigs.get('base');
    const testnet = defaultConfigs.get('testnet');

    return {
      '0.24.0': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            // Update images
            options.core.docker.image = base.get('core.docker.image');

            options.core.sentinel.docker.image = base.get('core.sentinel.docker.image');

            options.dashmate.helper.docker.image = base.get('dashmate.helper.docker.image');

            options.platform.drive.tenderdash.docker.image = base.get('platform.drive.tenderdash.docker.image');

            options.platform.drive.abci.docker.image = base.get('platform.drive.abci.docker.image');

            options.platform.dapi.api.docker.image = base.get('platform.dapi.api.docker.image');

            options.platform.dapi.envoy.docker.image = base.get('platform.dapi.envoy.docker.image');
          });

        return configFile;
      },
      '0.24.12': (configFile) => {
        let i = 0;
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            // Update dashmate helper port
            options.dashmate.helper.api.port = base.get('dashmate.helper.api.port');

            // Add pprof config
            options.platform.drive.tenderdash.pprof = base.get('platform.drive.tenderdash.pprof');

            // Set different ports for local netwrok if exists
            if (options.group === 'local') {
              options.platform.drive.tenderdash.pprof.port += i * 100;

              i++;
            }
          });

        return configFile;
      },
      '0.24.13': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.core.docker.image = base.get('core.docker.image');
          });

        return configFile;
      },
      '0.24.15': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.docker.network.bindIp = base.get('docker.network.bindIp');

            if (options.network === 'testnet') {
              options.platform.drive.tenderdash
                .genesis.initial_core_chain_locked_height = testnet.get('platform.drive.tenderdash.genesis.initial_core_chain_locked_height');
            }
          });

        return configFile;
      },
      '0.24.16': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.dapi.envoy.docker = base.get('platform.dapi.envoy.docker');

            options.platform.dapi.api.docker.build = base.get('platform.dapi.api.docker.build');

            options.platform.drive.abci.docker.build = base.get('platform.drive.abci.docker.build');

            options.dashmate.helper.docker.build = base.get('dashmate.helper.docker.build');

            delete options.dashmate.helper.docker.image;
            delete options.core.reindex;

            if (options.network === 'testnet') {
              options.platform.drive.tenderdash.genesis.chain_id = testnet.get('platform.drive.tenderdash.genesis.chain_id');
            }
          });

        return configFile;
      },
      '0.24.17': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.docker.baseImage = base.get('docker.baseImage');
          });

        return configFile;
      },
      '0.24.20': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.core.docker.image = base.get('core.docker.image');
          });
        return configFile;
      },
      '0.24.22': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            if (options.core.masternode.enable) {
              options.platform.drive.tenderdash.mode = 'validator';
            } else {
              options.platform.drive.tenderdash.mode = 'full';
            }
          });
        return configFile;
      },
      '0.25.0-dev.18': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            delete options.core.sentinel;

            if ([NETWORK_LOCAL, NETWORK_TESTNET].includes(options.network)) {
              options.core.docker.image = base.get('core.docker.image');
            }
          });
        return configFile;
      },
      '0.25.0-dev.29': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            if (options.network !== NETWORK_MAINNET) {
              options.core.docker.image = base.get('core.docker.image');

              options.platform.dapi.api.docker.image = base.get('platform.dapi.api.docker.image');
              options.platform.drive.abci.docker.image = base.get('platform.drive.abci.docker.image');
              options.platform.drive.tenderdash.docker.image = base.get('platform.drive.tenderdash.docker.image');
            }

            if (options.platform.drive.abci.log.jsonFile.level === 'fatal') {
              options.platform.drive.abci.log.jsonFile.level = 'error';
            }

            if (options.platform.drive.abci.log.prettyFile.level === 'fatal') {
              options.platform.drive.abci.log.prettyFile.level = 'error';
            }

            if (options.network === NETWORK_TESTNET) {
              options.platform.drive.tenderdash.genesis.chain_id = testnet.get('platform.drive.tenderdash.genesis.chain_id');
              options.platform.drive.tenderdash
                .genesis.initial_core_chain_locked_height = testnet.get('platform.drive.tenderdash.genesis.initial_core_chain_locked_height');
            }

            if (defaultConfigs.has(name) && !options.platform.drive.tenderdash.metrics) {
              options.platform.drive.tenderdash.metrics = defaultConfigs.get(name).get('platform.drive.tenderdash.metrics');
            }
          });
        return configFile;
      },
      '0.25.0-dev.30': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            if (options.network === NETWORK_TESTNET) {
              options.platform.drive.tenderdash.p2p.seeds = testnet.get('platform.drive.tenderdash.p2p.seeds');
            }
          });
        return configFile;
      },
      '0.25.0-dev.32': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            if (options.network !== NETWORK_MAINNET) {
              options.core.docker.image = base.get('core.docker.image');
            }

            if (options.network === NETWORK_TESTNET) {
              options.platform.drive.tenderdash.genesis.chain_id = testnet.get('platform.drive.tenderdash.genesis.chain_id');
              options.platform.drive.tenderdash.genesis.genesis_time = testnet.get('platform.drive.tenderdash.genesis.genesis_time');
            }
          });
        return configFile;
      },
      '0.25.0-dev.33': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.drive.abci.epochTime = base.get('platform.drive.abci.epochTime');
            options.platform.drive.tenderdash.docker.image = base.get('platform.drive.tenderdash.docker.image');
            options.platform.drive.tenderdash.log.path = null;

            if (options.platform.drive.abci.log.jsonFile.level === 'fatal') {
              options.platform.drive.abci.log.jsonFile.level = 'error';
            }

            if (options.platform.drive.abci.log.prettyFile.level === 'fatal') {
              options.platform.drive.abci.log.prettyFile.level = 'error';
            }

            if (options.network === NETWORK_TESTNET) {
              options.platform.drive.tenderdash.genesis.chain_id = testnet.get('platform.drive.tenderdash.genesis.chain_id');
              options.platform.drive.tenderdash.genesis.genesis_time = testnet.get('platform.drive.tenderdash.genesis.genesis_time');
              options.platform.drive.tenderdash.genesis
                .initial_core_chain_locked_height = testnet.get('platform.drive.tenderdash.genesis.initial_core_chain_locked_height');
            }

            if (options.network !== NETWORK_MAINNET) {
              options.core.docker.image = base.get('core.docker.image');
            }
          });

        return configFile;
      },
      '0.25.3': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            if (options.network === NETWORK_TESTNET) {
              options.platform.drive.abci.epochTime = testnet.get('platform.drive.abci.epochTime');
            }
            options.platform.drive.abci.docker.image = base.get('platform.drive.abci.docker.image');
            options.platform.dapi.api.docker.image = base.get('platform.drive.abci.docker.image');
          });

        return configFile;
      },
    };
  }

  return getConfigFileMigrations;
}

module.exports = getConfigFileMigrationsFactory;
