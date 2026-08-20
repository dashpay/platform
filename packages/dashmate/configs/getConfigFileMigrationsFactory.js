/* eslint-disable no-param-reassign */
import fs from 'fs';
import lodash from 'lodash';
import path from 'path';

import {
  NETWORK_LOCAL,
  NETWORK_MAINNET,
  NETWORK_TESTNET,
  SSL_PROVIDERS,
} from '../src/constants.js';
import { stockImagePattern, historicalStockImagePattern } from '../src/config/stockImages.js';

/**
 * @param {HomeDir} homeDir
 * @param {DefaultConfigs} defaultConfigs
 * @returns {getConfigFileMigrations}
 */
export default function getConfigFileMigrationsFactory(homeDir, defaultConfigs) {
  /**
   * @typedef {function} getConfigFileMigrations
   * @returns {Object}
   */
  function getConfigFileMigrations() {
    const base = defaultConfigs.get('base');
    const testnet = defaultConfigs.get('testnet');
    const mainnet = defaultConfigs.get('mainnet');

    /**
     * @param {string} name
     * @param {string} group
     * @return {Config}
     */
    function getDefaultConfigByNameOrGroup(name, group) {
      let baseConfigName = name;
      if (group !== null && defaultConfigs.has(group)) {
        baseConfigName = group;
      } else if (!defaultConfigs.has(baseConfigName)) {
        baseConfigName = 'base';
      }

      return defaultConfigs.get(baseConfigName);
    }

    /**
     * Re-pin a platform image, but only where it still holds a tag a release
     * published.
     *
     * These re-pins existed to keep a version-derived image current, and they
     * used to overwrite whatever was there. An operator running their own build
     * lost it the first time they crossed one, long before any later migration
     * could tell their image apart from a stale default.
     *
     * @param {Object} docker - the service's docker options, if present
     * @param {string} repository - image repository the service is published under
     * @param {string} image - image to move to
     */
    function repinStockImage(docker, repository, image) {
      if (docker && historicalStockImagePattern(repository).test(docker.image)) {
        // eslint-disable-next-line no-param-reassign
        docker.image = image;
      }
    }

    function getDefaultConfigByNetwork(network) {
      if (network === NETWORK_MAINNET) {
        return defaultConfigs.get('mainnet');
      }
      if (network === NETWORK_TESTNET) {
        return defaultConfigs.get('testnet');
      }
      if (network === NETWORK_LOCAL) {
        return defaultConfigs.get('local');
      }

      return defaultConfigs.get('base');
    }

    return {
      '0.24.0': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            // Update images
            options.core.docker.image = base.getStored('core.docker.image');

            options.core.sentinel.docker.image = base.getStored('core.sentinel.docker.image');

            options.dashmate.helper.docker.image = base.getStored('dashmate.helper.docker.image');

            options.platform.drive.tenderdash.docker.image = base.getStored('platform.drive.tenderdash.docker.image');

            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', base.getStored('platform.drive.abci.docker.image'));

            if (options.platform?.dapi?.api && base.has('platform.dapi.api.docker.image')) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', base.getStored('platform.dapi.api.docker.image'));
            }

            options.platform.gateway.docker.image = base.getStored('platform.gateway.docker.image');
          });

        return configFile;
      },
      '0.24.12': (configFile) => {
        let i = 0;
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            // Update dashmate helper port
            options.dashmate.helper.api.port = base.getStored('dashmate.helper.api.port');

            // Add pprof config
            options.platform.drive.tenderdash.pprof = base.getStored('platform.drive.tenderdash.pprof');

            // Set different ports for local network if exists
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
            options.core.docker.image = base.getStored('core.docker.image');
          });

        return configFile;
      },
      '0.24.15': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.docker.network.bindIp = base.getStored('docker.network.bindIp');

            if (options.network === 'testnet') {
              options.platform.drive.tenderdash
                .genesis.initial_core_chain_locked_height = testnet.getStored('platform.drive.tenderdash.genesis.initial_core_chain_locked_height');
            }
          });

        return configFile;
      },
      '0.24.16': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.gateway.docker = base.getStored('platform.gateway.docker');

            if (options.platform?.dapi?.api && base.has('platform.dapi.api.docker.build')) {
              options.platform.dapi.api.docker.build = base.getStored('platform.dapi.api.docker.build');
            }

            options.platform.drive.abci.docker.build = base.getStored('platform.drive.abci.docker.build');

            options.dashmate.helper.docker.build = base.getStored('dashmate.helper.docker.build');

            delete options.dashmate.helper.docker.image;
            delete options.core.reindex;

            if (options.network === 'testnet') {
              options.platform.drive.tenderdash.genesis.chain_id = testnet.getStored('platform.drive.tenderdash.genesis.chain_id');
            }
          });

        return configFile;
      },
      '0.24.17': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.docker.baseImage = base.getStored('docker.baseImage');
          });

        return configFile;
      },
      '0.24.20': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.core.docker.image = base.getStored('core.docker.image');
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
              options.core.docker.image = base.getStored('core.docker.image');
            }
          });
        return configFile;
      },
      '0.25.0-dev.29': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            if (options.network !== NETWORK_MAINNET) {
              options.core.docker.image = base.getStored('core.docker.image');

              if (options.platform?.dapi?.api && base.has('platform.dapi.api.docker.image')) {
                repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', base.getStored('platform.dapi.api.docker.image'));
              }
              repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', base.getStored('platform.drive.abci.docker.image'));
              options.platform.drive.tenderdash.docker.image = base.getStored('platform.drive.tenderdash.docker.image');
            }

            if (options.platform.drive.abci.log.jsonFile.level === 'fatal') {
              options.platform.drive.abci.log.jsonFile.level = 'error';
            }

            if (options.platform.drive.abci.log.prettyFile.level === 'fatal') {
              options.platform.drive.abci.log.prettyFile.level = 'error';
            }

            if (options.network === NETWORK_TESTNET) {
              options.platform.drive.tenderdash.genesis.chain_id = testnet.getStored('platform.drive.tenderdash.genesis.chain_id');
              options.platform.drive.tenderdash
                .genesis.initial_core_chain_locked_height = testnet.getStored('platform.drive.tenderdash.genesis.initial_core_chain_locked_height');
            }

            if (defaultConfigs.has(name) && !options.platform.drive.tenderdash.metrics) {
              options.platform.drive.tenderdash.metrics = defaultConfigs.get(name)
                .getStored('platform.drive.tenderdash.metrics');
            }
          });
        return configFile;
      },
      '0.25.0-dev.30': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            if (options.network === NETWORK_TESTNET) {
              options.platform.drive.tenderdash.p2p.seeds = testnet.getStored('platform.drive.tenderdash.p2p.seeds');
            }
          });
        return configFile;
      },
      '0.25.0-dev.32': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            if (options.network !== NETWORK_MAINNET) {
              options.core.docker.image = base.getStored('core.docker.image');
            }

            if (options.network === NETWORK_TESTNET) {
              options.platform.drive.tenderdash.genesis.chain_id = testnet.getStored('platform.drive.tenderdash.genesis.chain_id');
              options.platform.drive.tenderdash.genesis.genesis_time = '2024-07-17T17:15:00.000Z';
            }
          });
        return configFile;
      },
      '0.25.0-dev.33': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.drive.abci.epochTime = base.getStored('platform.drive.abci.epochTime');
            options.platform.drive.tenderdash.docker.image = base.getStored('platform.drive.tenderdash.docker.image');
            options.platform.drive.tenderdash.log.path = null;

            if (options.platform.drive.abci.log.jsonFile.level === 'fatal') {
              options.platform.drive.abci.log.jsonFile.level = 'error';
            }

            if (options.platform.drive.abci.log.prettyFile.level === 'fatal') {
              options.platform.drive.abci.log.prettyFile.level = 'error';
            }

            if (options.network === NETWORK_TESTNET) {
              options.platform.drive.tenderdash.genesis.chain_id = testnet.getStored('platform.drive.tenderdash.genesis.chain_id');
              options.platform.drive.tenderdash.genesis.genesis_time = '2024-07-17T17:15:00.000Z';
              options.platform.drive.tenderdash.genesis
                .initial_core_chain_locked_height = testnet.getStored('platform.drive.tenderdash.genesis.initial_core_chain_locked_height');
            }

            if (options.network !== NETWORK_MAINNET) {
              options.core.docker.image = base.getStored('core.docker.image');
            }
          });

        return configFile;
      },
      '0.25.3': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            if (options.network === NETWORK_TESTNET && name !== 'base') {
              options.platform.drive.abci.epochTime = testnet.getStored('platform.drive.abci.epochTime');
            }
            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', base.getStored('platform.drive.abci.docker.image'));
            if (options.platform?.dapi?.api && base.has('platform.dapi.api.docker.image')) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', base.getStored('platform.dapi.api.docker.image'));
            }
          });

        return configFile;
      },
      '0.25.4': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            delete options.platform.drive.abci.log;

            options.platform.drive.abci.logs = base.getStored('platform.drive.abci.logs');
          });

        return configFile;
      },
      '0.25.7': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            if (options.network !== NETWORK_MAINNET) {
              const filenames = ['private.key', 'bundle.crt', 'bundle.csr', 'csr.pem'];

              for (const filename of filenames) {
                const oldFilePath = homeDir.joinPath('ssl', name, filename);
                const newFilePath = homeDir.joinPath(
                  name,
                  'platform',
                  'dapi',
                  'envoy',
                  'ssl',
                  filename,
                );

                if (fs.existsSync(oldFilePath)) {
                  fs.mkdirSync(path.dirname(newFilePath), { recursive: true });
                  fs.copyFileSync(oldFilePath, newFilePath);
                  fs.rmSync(oldFilePath, { recursive: true });
                }
              }
            }
          });

        if (fs.existsSync(homeDir.joinPath('ssl'))) {
          fs.rmSync(homeDir.joinPath('ssl'), { recursive: true });
        }

        return configFile;
      },
      '0.25.11': (configFile) => {
        if (configFile.configs.base) {
          configFile.configs.base.core.docker.image = base.getStored('core.docker.image');
        }
        if (configFile.configs.local) {
          configFile.configs.local.platform.dapi.envoy.ssl.provider = SSL_PROVIDERS.SELF_SIGNED;
        }

        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.drive.tenderdash.log.level = 'info';

            if (options.network !== NETWORK_MAINNET && options.network !== NETWORK_TESTNET) {
              options.core.docker.image = base.getStored('core.docker.image');
            }

            options.core.docker.commandArgs = [];
          });

        return configFile;
      },
      '0.25.12': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            options.platform.drive.tenderdash.docker.image = base.getStored('platform.drive.tenderdash.docker.image');

            if (options.network === NETWORK_TESTNET) {
              options.core.docker.image = base.getStored('core.docker.image');

              if (name !== base.getName()) {
                options.platform.drive.tenderdash.genesis.chain_id = testnet.getStored('platform.drive.tenderdash.genesis.chain_id');
                options.platform.drive.tenderdash.genesis.initial_core_chain_locked_height = 14000;
                options.platform.drive.tenderdash.genesis.genesis_time = '2024-07-17T17:15:00.000Z';
              }
            }
          });

        return configFile;
      },
      '0.25.16-rc.1': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            options.core.insight = base.getStored('core.insight');
            options.core.docker.image = base.getStored('core.docker.image');

            if (options.network === NETWORK_TESTNET && name !== base.getName()) {
              options.platform.drive.tenderdash.genesis.chain_id = testnet.getStored('platform.drive.tenderdash.genesis.chain_id');
              options.platform.drive.tenderdash.genesis.initial_core_chain_locked_height = 1400;
              options.platform.drive.tenderdash.genesis.genesis_time = '2024-07-17T17:15:00.000Z';
            }
          });

        return configFile;
      },
      '0.25.16-rc.5': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            if (options.network === NETWORK_TESTNET && name !== base.getName()) {
              options.platform.drive.tenderdash.genesis.chain_id = testnet.getStored('platform.drive.tenderdash.genesis.chain_id');
              options.platform.drive.tenderdash.genesis.initial_core_chain_locked_height = 1400;
              options.platform.drive.tenderdash.genesis.genesis_time = '2024-07-17T17:15:00.000Z';
            }
          });

        return configFile;
      },
      '0.25.16-rc.6': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.core.docker.image = base.getStored('core.docker.image');
          });

        return configFile;
      },
      '0.25.16-rc.7': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.drive.tenderdash.docker.image = base.getStored('platform.drive.tenderdash.docker.image');

            delete options.docker.network.bindIp;

            options.core.p2p.host = base.getStored('core.p2p.host');
            options.core.rpc.host = base.getStored('core.rpc.host');
            options.platform.dapi.envoy.http.host = '0.0.0.0';
            options.platform.drive.tenderdash.p2p.host = base.getStored('platform.drive.tenderdash.p2p.host');
            options.platform.drive.tenderdash.rpc.host = base.getStored('platform.drive.tenderdash.rpc.host');
            options.platform.drive.tenderdash.metrics.host = base.getStored('platform.drive.tenderdash.metrics.host');
          });

        return configFile;
      },
      '0.25.19': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.drive.tenderdash.docker.image = base.getStored('platform.drive.tenderdash.docker.image');
          });

        return configFile;
      },
      '0.25.20': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            options.platform.dapi.envoy.http.connectTimeout = '5s';
            options.platform.dapi.envoy.http.responseTimeout = '15s';

            options.platform.drive.tenderdash.rpc.maxOpenConnections = base.getStored('platform.drive.tenderdash.rpc.maxOpenConnections');

            let defaultConfigName = 'base';
            if (options.group === 'local' || name === 'local') {
              defaultConfigName = 'local';
            }
            const defaultConfig = defaultConfigs.get(defaultConfigName);

            options.platform.drive.tenderdash.p2p.flushThrottleTimeout = defaultConfig.getStored('platform.drive.tenderdash.p2p.flushThrottleTimeout');
            options.platform.drive.tenderdash.p2p.maxPacketMsgPayloadSize = defaultConfig.getStored('platform.drive.tenderdash.p2p.maxPacketMsgPayloadSize');
            options.platform.drive.tenderdash.p2p.sendRate = defaultConfig.getStored('platform.drive.tenderdash.p2p.sendRate');
            options.platform.drive.tenderdash.p2p.recvRate = defaultConfig.getStored('platform.drive.tenderdash.p2p.recvRate');

            options.platform.drive.tenderdash.mempool = lodash.clone(base.getStored('platform.drive.tenderdash.mempool'));
            options.platform.drive.tenderdash.consensus.peer = base.getStored('platform.drive.tenderdash.consensus.peer');
            options.platform.drive.tenderdash.consensus.unsafeOverride = base.getStored('platform.drive.tenderdash.consensus.unsafeOverride');
          });

        return configFile;
      },
      '0.25.22': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            if (options.platform?.dapi?.api && base.has('platform.dapi.api.docker.deploy')) {
              options.platform.dapi.api.docker.deploy = base.getStored('platform.dapi.api.docker.deploy');
            }
          });

        return configFile;
      },
      '1.0.0-dev.2': (configFile) => {
        const consensusParams = {
          block: {
            max_bytes: '2097152',
            max_gas: '57631392000',
            time_iota_ms: '5000',
          },
          evidence: {
            max_age: '100000',
            max_age_num_blocks: '100000',
            max_age_duration: '172800000000000',
          },
          validator: {
            pub_key_types: ['bls12381'],
          },
          timeout: {
            propose: '50000000000',
            propose_delta: '5000000000',
            vote: '10000000000',
            vote_delta: '1000000000',
          },
          synchrony: {
            message_delay: '70000000000',
            precision: '1000000000',
          },
          abci: {
            recheck_tx: true,
          },
          version: {
            app_version: '1',
          },
        };

        const genesis = {
          base: {
            consensus_params: lodash.cloneDeep(consensusParams),
          },
          local: {
            consensus_params: lodash.cloneDeep(consensusParams),
          },
          testnet: {
            chain_id: 'dash-testnet-51',
            validator_quorum_type: 6,
            consensus_params: lodash.cloneDeep(consensusParams),
          },
          mainnet: {
            chain_id: 'evo1',
            validator_quorum_type: 4,
            consensus_params: lodash.cloneDeep(consensusParams),
          },
        };

        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            if (genesis[name]) {
              options.platform.drive.tenderdash.genesis = genesis[name];
            }

            if (options.platform?.dapi?.api && base.has('platform.dapi.api.docker.deploy')) {
              options.platform.dapi.api.docker.deploy = base.getStored('platform.dapi.api.docker.deploy');
            }

            let baseConfigName = name;
            if (options.group !== null && defaultConfigs.has(options.group)) {
              baseConfigName = options.group;
            } else if (!defaultConfigs.has(baseConfigName)) {
              baseConfigName = 'testnet';
            }

            options.platform.drive.abci.chainLock = defaultConfigs.get(baseConfigName)
              .getStored('platform.drive.abci.chainLock');
          });

        return configFile;
      },
      '1.0.0-dev.4': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            const defaultConfig = getDefaultConfigByNameOrGroup(name, options.group);
            options.core.docker.image = defaultConfig.getStored('core.docker.image');

            options.platform.drive.tenderdash.docker.image = defaultConfig.getStored('platform.drive.tenderdash.docker.image');
          });

        return configFile;
      },
      '1.0.0-dev.5': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.drive.tenderdash.mempool.cacheSize = base.getStored('platform.drive.tenderdash.mempool.cacheSize');
          });

        return configFile;
      },
      '1.0.0-dev.6': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            options.platform.drive.abci.tokioConsole = base.getStored('platform.drive.abci.tokioConsole');

            const defaultConfig = getDefaultConfigByNameOrGroup(name, options.group);
            options.platform.drive.tenderdash.docker.image = defaultConfig.getStored('platform.drive.tenderdash.docker.image');
          });

        return configFile;
      },
      '1.0.0-dev.7': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            if (options.network === NETWORK_TESTNET && name !== 'base') {
              options.platform.drive.tenderdash.genesis = lodash.cloneDeep(testnet.getStored('platform.drive.tenderdash.genesis'));
            }

            const defaultConfig = getDefaultConfigByNameOrGroup(name, options.group);
            options.core.docker.image = defaultConfig.getStored('core.docker.image');
          });

        return configFile;
      },
      '1.0.0-dev.8': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            const defaultConfig = getDefaultConfigByNameOrGroup(name, options.group);
            options.core.docker.image = defaultConfig.getStored('core.docker.image');
          });

        return configFile;
      },
      '1.0.0-dev.9': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            options.platform.drive.tenderdash.docker.image = base.getStored('platform.drive.tenderdash.docker.image');

            const defaultConfig = getDefaultConfigByNameOrGroup(name, options.group);
            options.platform.drive.tenderdash.mempool.timeoutCheckTx = defaultConfig.getStored('platform.drive.tenderdash.mempool.timeoutCheckTx');
            options.platform.drive.tenderdash.mempool.txEnqueueTimeout = defaultConfig.getStored('platform.drive.tenderdash.mempool.txEnqueueTimeout');
            options.platform.drive.tenderdash.mempool.txSendRateLimit = defaultConfig.getStored('platform.drive.tenderdash.mempool.txSendRateLimit');
            options.platform.drive.tenderdash.mempool.txRecvRateLimit = defaultConfig.getStored('platform.drive.tenderdash.mempool.txRecvRateLimit');
            options.platform.drive.tenderdash.rpc.timeoutBroadcastTx = defaultConfig.getStored('platform.drive.tenderdash.rpc.timeoutBroadcastTx');
          });

        return configFile;
      },
      '1.0.0-dev.10': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.drive.tenderdash.docker.image = base.getStored('platform.drive.tenderdash.docker.image');
          });

        return configFile;
      },
      '1.0.0-dev.12': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            // Update tenderdash config
            options.platform.drive.tenderdash.docker.image = base.getStored('platform.drive.tenderdash.docker.image');
            options.platform.drive.tenderdash.mempool.maxConcurrentCheckTx = base.getStored('platform.drive.tenderdash.mempool.maxConcurrentCheckTx');

            // Add metrics to Drive ABCI
            options.platform.drive.abci.metrics = base.getStored('platform.drive.abci.metrics');

            // Envoy -> Gateway
            if (options.platform.dapi.envoy) {
              options.platform.gateway = lodash.cloneDeep(options.platform.dapi.envoy);

              // add new options
              options.platform.gateway.maxConnections = base.getStored('platform.gateway.maxConnections');
              options.platform.gateway.maxHeapSizeInBytes = base.getStored('platform.gateway.maxHeapSizeInBytes');
              options.platform.gateway.metrics = base.getStored('platform.gateway.metrics');
              options.platform.gateway.admin = base.getStored('platform.gateway.admin');
              options.platform.gateway.upstreams = base.getStored('platform.gateway.upstreams');
              options.platform.gateway.log = base.getStored('platform.gateway.log');

              // http -> listeners
              options.platform.gateway.listeners = lodash.cloneDeep(
                base.getStored('platform.gateway.listeners'),
              );

              options.platform.gateway.listeners.dapiAndDrive.host = options.platform.dapi.envoy
                .http.host;
              options.platform.gateway.listeners.dapiAndDrive.port = options.platform.dapi.envoy
                .http.port;

              delete options.platform.gateway.http;

              // update rate limiter
              options.platform.gateway.rateLimiter.docker = base.getStored('platform.gateway.rateLimiter.docker');
              options.platform.gateway.rateLimiter.unit = base.getStored('platform.gateway.rateLimiter.unit');
              options.platform.gateway.rateLimiter.requestsPerUnit = base.getStored('platform.gateway.rateLimiter.requestsPerUnit');
              options.platform.gateway.rateLimiter.blacklist = base.getStored('platform.gateway.rateLimiter.blacklist');
              options.platform.gateway.rateLimiter.whitelist = base.getStored('platform.gateway.rateLimiter.whitelist');
              options.platform.gateway.rateLimiter.metrics = base.getStored('platform.gateway.rateLimiter.metrics');

              delete options.platform.gateway.rateLimiter.fillInterval;
              delete options.platform.gateway.rateLimiter.maxTokens;
              delete options.platform.gateway.rateLimiter.tokensPerFill;

              // delete envoy
              delete options.platform.dapi.envoy;

              // update image
              options.platform.gateway.docker.image = base.getStored('platform.gateway.docker.image');
            }

            // rename non conventional field
            if (options.platform.drive.abci.tokioConsole.retention_secs) {
              options.platform.drive.abci.tokioConsole.retention = options.platform.drive.abci
                .tokioConsole.retention_secs;
              delete options.platform.drive.abci.tokioConsole.retention_secs;
            }

            // move SSL files
            if (options.network !== NETWORK_MAINNET) {
              const filenames = ['private.key', 'bundle.crt', 'bundle.csr', 'csr.pem'];

              for (const filename of filenames) {
                const oldFilePath = homeDir.joinPath(
                  name,
                  'platform',
                  'dapi',
                  'envoy',
                  'ssl',
                  filename,
                );
                const newFilePath = homeDir.joinPath(
                  name,
                  'platform',
                  'gateway',
                  'ssl',
                  filename,
                );

                if (fs.existsSync(oldFilePath)) {
                  fs.mkdirSync(path.dirname(newFilePath), { recursive: true });
                  fs.copyFileSync(oldFilePath, newFilePath);
                  fs.rmSync(oldFilePath, { recursive: true });
                }
              }
            }
          });

        return configFile;
      },
      '1.0.0-dev.16': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            // Update Drive's quorum configuration
            if (name === 'base') {
              options.network = NETWORK_MAINNET;
            }

            const networkConfig = getDefaultConfigByNetwork(options.network);

            options.platform.drive.abci.chainLock.quorum = {
              llmqType: networkConfig.getStored('platform.drive.abci.chainLock.quorum.llmqType'),
              dkgInterval: networkConfig.getStored('platform.drive.abci.chainLock.quorum.dkgInterval'),
              activeSigners: networkConfig.getStored('platform.drive.abci.chainLock.quorum.activeSigners'),
              rotation: networkConfig.getStored('platform.drive.abci.chainLock.quorum.rotation'),
            };

            delete options.platform.drive.abci.chainLock.llmqType;
            delete options.platform.drive.abci.chainLock.llmqSize;
            delete options.platform.drive.abci.chainLock.dkgInterval;

            options.platform.drive.abci.validatorSet.quorum = {
              llmqType: networkConfig.getStored('platform.drive.abci.validatorSet.quorum.llmqType'),
              dkgInterval: networkConfig.getStored('platform.drive.abci.validatorSet.quorum.dkgInterval'),
              activeSigners: networkConfig.getStored('platform.drive.abci.validatorSet.quorum.activeSigners'),
              rotation: networkConfig.getStored('platform.drive.abci.validatorSet.quorum.rotation'),
            };

            delete options.platform.drive.abci.validatorSet.llmqType;

            options.platform.drive.abci.instantLock = {
              quorum: {
                llmqType: networkConfig.getStored('platform.drive.abci.instantLock.quorum.llmqType'),
                dkgInterval: networkConfig.getStored('platform.drive.abci.instantLock.quorum.dkgInterval'),
                activeSigners: networkConfig.getStored('platform.drive.abci.instantLock.quorum.activeSigners'),
                rotation: networkConfig.getStored('platform.drive.abci.instantLock.quorum.rotation'),
              },
            };
          });

        return configFile;
      },
      '1.0.0-dev.17': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            options.platform.drive.tenderdash.docker.image = base.getStored('platform.drive.tenderdash.docker.image');
            options.platform.drive.abci.grovedbVisualizer = base.getStored('platform.drive.abci.grovedbVisualizer');

            // Update Core image
            options.core.docker.image = getDefaultConfigByNameOrGroup(name, options.group)
              .getStored('core.docker.image');

            // Update Core RPC auth configuration
            options.core.rpc.users = base.getStored('core.rpc.users');
            options.core.rpc.users.dashmate.password = options.core.rpc.password;

            delete options.core.rpc.user;
            delete options.core.rpc.password;
          });
        return configFile;
      },
      '1.0.0-beta.4': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            // Update Core image
            options.core.docker.image = getDefaultConfigByNameOrGroup(name, options.group)
              .getStored('core.docker.image');

            options.core.devnet.llmq = base.getStored('core.devnet.llmq');

            if (options.network === NETWORK_TESTNET) {
              options.platform.drive.tenderdash.genesis = lodash.cloneDeep(testnet.getStored('platform.drive.tenderdash.genesis'));
            }
          });
        return configFile;
      },
      '1.0.0-rc.1': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            delete options.platform.dpns;
            delete options.platform.dashpay;
            delete options.platform.featureFlags;
            delete options.platform.masternodeRewardShares;
            delete options.platform.withdrawals;

            // Update tenderdash image
            options.platform.drive.tenderdash.docker.image = base.getStored('platform.drive.tenderdash.docker.image');

            // Replace quorumsign with qurumplatformsign in Core RPC Tenderdash auth whitelist
            options.core.rpc.users.tenderdash.whitelist = base.getStored('core.rpc.users.tenderdash.whitelist');
          });
        return configFile;
      },
      '1.0.0-rc.2': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            if (options.network === NETWORK_TESTNET) {
              options.platform.drive.tenderdash.genesis = lodash.cloneDeep(testnet.getStored('platform.drive.tenderdash.genesis'));
            }

            // Update tenderdash image
            options.platform.drive.tenderdash.docker.image = base.getStored('platform.drive.tenderdash.docker.image');
            options.core.rpc.users.drive_consensus.whitelist = base.getStored('core.rpc.users.drive_consensus.whitelist');
          });
        return configFile;
      },
      '1.0.0': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            if (name === 'base') {
              options.platform.drive.tenderdash.mempool = base.getStored('platform.drive.tenderdash.mempool');
              options.platform.drive.tenderdash.genesis = base.getStored('platform.drive.tenderdash.genesis');
            } else if (options.network === NETWORK_MAINNET) {
              options.platform.drive.tenderdash.p2p = mainnet.getStored('platform.drive.tenderdash.p2p');
              options.platform.drive.tenderdash.mempool = mainnet.getStored('platform.drive.tenderdash.mempool');
              options.platform.drive.tenderdash.genesis = mainnet.getStored('platform.drive.tenderdash.genesis');

              if (options.platform.drive.tenderdash.node.id !== null) {
                options.platform.enable = true;
              }
            }

            // Update tenderdash image
            options.platform.drive.tenderdash.docker.image = base.getStored('platform.drive.tenderdash.docker.image');
            options.core.docker.image = base.getStored('core.docker.image');
          });
        return configFile;
      },
      '1.0.2': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.core.indexes = [];
            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', 'dashpay/drive:1');
            if (options.platform?.dapi?.api) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', 'dashpay/dapi:1');
            }
          });
        return configFile;
      },
      '1.1.0-dev.1': (configFile) => {
        const consensusParams = {
          block: {
            max_bytes: '2097152',
            max_gas: '57631392000',
            time_iota_ms: '5000',
          },
          evidence: {
            max_age: '100000',
            max_age_num_blocks: '100000',
            max_age_duration: '172800000000000',
          },
          validator: {
            pub_key_types: ['bls12381'],
          },
          timeout: {
            propose: '50000000000',
            propose_delta: '5000000000',
            vote: '10000000000',
            vote_delta: '1000000000',
          },
          synchrony: {
            message_delay: '70000000000',
            precision: '1000000000',
          },
          abci: {
            recheck_tx: true,
          },
          version: {
            app_version: '1',
          },
        };

        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            if (name === 'local') {
              options.platform.drive.abci.epochTime = 1200;
            }

            if (options.network === NETWORK_MAINNET && name !== 'base') {
              options.platform.drive.tenderdash.p2p.seeds = mainnet.getStored('platform.drive.tenderdash.p2p.seeds');
            }

            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', 'dashpay/drive:1-dev');
            if (options.platform?.dapi?.api) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', 'dashpay/dapi:1-dev');
              options.platform.dapi.api.waitForStResultTimeout = 120000;
            }

            options.platform.gateway.listeners.dapiAndDrive.waitForStResultTimeout = '125s';

            options.platform.drive.tenderdash.p2p.maxConnections = 64;
            options.platform.drive.tenderdash.p2p.maxOutgoingConnections = 30;

            if (defaultConfigs.has(name)) {
              options.platform.drive.tenderdash.genesis
                .consensus_params = lodash.cloneDeep(consensusParams);
            }

            options.platform.drive.tenderdash.docker.image = base.getStored('platform.drive.tenderdash.docker.image');
          });
        return configFile;
      },
      '1.1.0': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', 'dashpay/drive:1');
            if (options.platform?.dapi?.api) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', 'dashpay/dapi:1');
            }

            if (options.network === NETWORK_TESTNET) {
              options.platform.drive.abci.proposer = {
                txProcessingTimeLimit: 5000,
              };
              options.platform.drive.tenderdash.mempool.timeoutCheckTx = '3s';
              options.platform.drive.tenderdash.mempool.txEnqueueTimeout = '30ms';
              options.platform.drive.tenderdash.mempool.txSendRateLimit = 100;
              options.platform.drive.tenderdash.mempool.txRecvRateLimit = 120;
              options.platform.drive.tenderdash.mempool.ttlDuration = '24h';
              options.platform.drive.tenderdash.mempool.ttlNumBlocks = 0;
            } else if (options.network === NETWORK_MAINNET && name !== 'base') {
              options.platform.drive.abci.proposer = {
                txProcessingTimeLimit: 5000,
              };
              options.platform.drive.tenderdash.mempool.ttlDuration = '24h';
              options.platform.drive.tenderdash.mempool.ttlNumBlocks = 0;
            } else {
              options.platform.drive.tenderdash.mempool.ttlDuration = '0s';
              options.platform.drive.tenderdash.mempool.ttlNumBlocks = 0;
              options.platform.drive.abci.proposer = {
                txProcessingTimeLimit: null,
              };
            }
          });
        return configFile;
      },
      '1.1.1': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.drive.tenderdash.docker.image = 'dashpay/tenderdash:1.2.0';
            if (options.network === NETWORK_TESTNET) {
              options.platform.drive.tenderdash.genesis.chain_id = 'dash-testnet-51';
            }
          });
        return configFile;
      },
      '1.2.0-rc.1': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            options.platform.drive.tenderdash.docker.image = 'dashpay/tenderdash:1';
            if (options.network === NETWORK_MAINNET && name !== 'base') {
              options.platform.drive.tenderdash.genesis.chain_id = 'evo1';
            }
            if (options.network === NETWORK_TESTNET) {
              delete options.platform.drive.tenderdash.genesis.initial_core_chain_locked_height;
            }
          });
        return configFile;
      },
      '1.3.0-dev.3': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', 'dashpay/drive:1-dev');
            if (options.platform?.dapi?.api) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', 'dashpay/dapi:1-dev');
            }

            // Update core log settings
            options.core.log.filePath = null;
            options.core.log.debug = {
              enabled: false,
              ips: !!options.core.logIps,
              sourceLocations: false,
              threadNames: false,
              timeMicros: false,
              includeOnly: [],
              exclude: [],
            };

            // If debug log was enabled
            if (options.core.log.file.categories.length > 0) {
              options.core.log.filePath = options.core.log.file.path;
              options.core.log.debug.enabled = true;

              if (!options.core.log.file.categories.includes('all')) {
                options.core.log.debug.includeOnly = options.core.log.file.categories;
              }
            }

            delete options.core.log.file;
            delete options.core.logIps;
          });
        return configFile;
      },
      '1.3.0-dev.6': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.drive.tenderdash.docker.image = 'dashpay/tenderdash:fix-wrong-proposer-at-round';
          });
        return configFile;
      },
      '1.3.0': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.drive.tenderdash.docker.image = 'dashpay/tenderdash:1.3';
            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', 'dashpay/drive:1');
            if (options.platform?.dapi?.api) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', 'dashpay/dapi:1');
            }
          });
        return configFile;
      },
      '1.4.0-dev.1': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.drive.tenderdash.docker.image = 'dashpay/tenderdash:1.3';
            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', 'dashpay/drive:1-dev');
            if (options.platform?.dapi?.api) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', 'dashpay/dapi:1-dev');
            }
          });
        return configFile;
      },
      '1.4.0-dev.4': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            if (name === 'base' || name === 'local') {
              delete options.platform.drive.tenderdash.genesis.consensus_params.version;
            } else if (options.network === NETWORK_TESTNET) {
              options.platform.drive.tenderdash.genesis.consensus_params.version = {
                app_version: '1',
              };
            }
          });
        return configFile;
      },
      '1.4.0': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', 'dashpay/drive:1');
            if (options.platform?.dapi?.api) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', 'dashpay/dapi:1');
            }
          });
        return configFile;
      },
      '1.5.0': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            if (options.network === NETWORK_MAINNET && name !== 'base') {
              options.platform.drive.tenderdash.p2p.seeds = mainnet.getStored('platform.drive.tenderdash.p2p.seeds');
            }

            if (options.network === NETWORK_TESTNET && name !== 'base') {
              options.platform.drive.tenderdash.p2p.seeds = testnet.getStored('platform.drive.tenderdash.p2p.seeds');
            }
          });
        return configFile;
      },
      '1.6.0': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', 'dashpay/drive:1-dev');
            if (options.platform?.dapi?.api) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', 'dashpay/dapi:1-dev');
            }
          });
        return configFile;
      },
      '1.6.1': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', 'dashpay/drive:1');
            if (options.platform?.dapi?.api) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', 'dashpay/dapi:1');
            }
          });
        return configFile;
      },
      '1.7.0': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.core.docker.image = 'dashpay/dashd:22';
            options.platform.drive.tenderdash.docker.image = 'dashpay/tenderdash:1';
          });
        return configFile;
      },
      '1.8.0': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            delete options.core.miner.mediantime;

            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', 'dashpay/drive:1');
            if (options.platform?.dapi?.api) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', 'dashpay/dapi:1');
            }
          });
        return configFile;
      },
      '2.0.0-dev.1': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            delete options.core.miner.mediantime;

            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', 'dashpay/drive:2-dev');
            if (options.platform?.dapi?.api) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', 'dashpay/dapi:2-dev');
            }
          });
        return configFile;
      },
      '2.0.0-rc.1': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            delete options.core.miner.mediantime;

            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', 'dashpay/drive:2-rc');
            if (options.platform?.dapi?.api) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', 'dashpay/dapi:2-rc');
            }
          });
        return configFile;
      },
      '2.0.0': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            delete options.core.miner.mediantime;

            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', 'dashpay/drive:2');
            if (options.platform?.dapi?.api) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', 'dashpay/dapi:2');
            }
          });
        return configFile;
      },
      '2.0.2-rc.1': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            if (options.network === NETWORK_TESTNET && name !== 'base') {
              options.platform.drive.tenderdash.genesis.consensus_params = lodash.cloneDeep(testnet.getStored('platform.drive.tenderdash.genesis.consensus_params'));
            }
          });
        return configFile;
      },
      '2.1.0-dev.1': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            // Add ZMQ configuration if it doesn't exist
            if (!options.core.zmq) {
              options.core.zmq = base.getStored('core.zmq');
            }

            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', 'dashpay/drive:2-dev');
            if (options.platform?.dapi?.api) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', 'dashpay/dapi:2-dev');
            }
            options.platform.drive.tenderdash.docker.image = 'dashpay/tenderdash:1-dev';
          });
        return configFile;
      },
      '2.1.0-dev.9': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            const defaultConfig = getDefaultConfigByNameOrGroup(name, options.group);

            if (!options.platform.dapi.rsDapi) {
              options.platform.dapi.rsDapi = lodash.cloneDeep(defaultConfig.getStored('platform.dapi.rsDapi'));
              return;
            }

            const defaultMetrics = defaultConfig.getStored('platform.dapi.rsDapi.metrics');

            if (options.platform.dapi.rsDapi.healthCheck) {
              options.platform.dapi.rsDapi.metrics = lodash.cloneDeep(
                options.platform.dapi.rsDapi.healthCheck,
              );
              delete options.platform.dapi.rsDapi.healthCheck;
            }

            if (!options.platform.dapi.rsDapi.metrics) {
              options.platform.dapi.rsDapi.metrics = lodash.cloneDeep(defaultMetrics);
            }

            if (typeof options.platform.dapi.rsDapi.metrics.host === 'undefined') {
              options.platform.dapi.rsDapi.metrics.host = defaultMetrics.host;
            }

            if (typeof options.platform.dapi.rsDapi.metrics.port === 'undefined') {
              options.platform.dapi.rsDapi.metrics.port = defaultMetrics.port;
            }

            if (!options.platform.dapi.rsDapi.logs) {
              options.platform.dapi.rsDapi.logs = lodash.cloneDeep(defaultConfig.getStored('platform.dapi.rsDapi.logs'));
            }

            if (typeof options.platform.dapi.rsDapi.logs.level === 'undefined') {
              options.platform.dapi.rsDapi.logs.level = defaultConfig.getStored('platform.dapi.rsDapi.logs.level');
            }

            if (typeof options.platform.dapi.rsDapi.logs.jsonFormat === 'undefined') {
              options.platform.dapi.rsDapi.logs.jsonFormat = defaultConfig.getStored('platform.dapi.rsDapi.logs.jsonFormat');
            }

            if (typeof options.platform.dapi.rsDapi.logs.accessLogPath === 'undefined') {
              options.platform.dapi.rsDapi.logs.accessLogPath = defaultConfig.getStored('platform.dapi.rsDapi.logs.accessLogPath');
            }

            if (typeof options.platform.dapi.rsDapi.logs.accessLogFormat === 'undefined') {
              options.platform.dapi.rsDapi.logs.accessLogFormat = defaultConfig.getStored('platform.dapi.rsDapi.logs.accessLogFormat');
            }
          });

        return configFile;
      },
      '2.1.0-pr.2716.1': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            const defaultConfig = getDefaultConfigByNameOrGroup(name, options.group);

            if (options.platform?.dapi?.api && defaultConfig.has('platform.dapi.api.docker.image')) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', defaultConfig
                .getStored('platform.dapi.api.docker.image'));
            }

            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', defaultConfig
              .getStored('platform.drive.abci.docker.image'));

            if (options.platform.dapi.rsDapi
              && defaultConfig.has('platform.dapi.rsDapi.docker.image')) {
              repinStockImage(options.platform?.dapi?.rsDapi?.docker, 'dashpay/rs-dapi', defaultConfig
                .getStored('platform.dapi.rsDapi.docker.image'));
            }

            if (options.platform.drive.tenderdash
              && defaultConfig.has('platform.drive.tenderdash.docker.image')) {
              options.platform.drive.tenderdash.docker.image = defaultConfig
                .getStored('platform.drive.tenderdash.docker.image');
            }
          });

        return configFile;
      },
      '2.1.0-rc.1': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', 'dashpay/drive:2-rc');
            if (options.platform?.dapi?.api) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', 'dashpay/dapi:2-rc');
            }
            repinStockImage(options.platform?.dapi?.rsDapi?.docker, 'dashpay/rs-dapi', 'dashpay/rs-dapi:2-rc');
            options.platform.drive.tenderdash.docker.image = 'dashpay/tenderdash:1.5';
          });

        return configFile;
      },
      '2.1.0': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', 'dashpay/drive:2');
            if (options.platform?.dapi?.api) {
              repinStockImage(options.platform?.dapi?.api?.docker, 'dashpay/dapi', 'dashpay/dapi:2');
            }
            repinStockImage(options.platform?.dapi?.rsDapi?.docker, 'dashpay/rs-dapi', 'dashpay/rs-dapi:2');
          });

        return configFile;
      },
      '3.0.0': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            const defaultConfig = getDefaultConfigByNameOrGroup(name, options.group);
            const isLocal = options.network === NETWORK_LOCAL || name === 'local';
            const isTestnet = options.network === NETWORK_TESTNET || name === 'testnet';

            // --- ZMQ configuration ---
            if (!options.core.zmq) {
              options.core.zmq = lodash.cloneDeep(defaultConfig.getStored('core.zmq'));
            } else {
              options.core.zmq = lodash.cloneDeep(options.core.zmq);
            }

            if (typeof options.core.zmq.port === 'undefined') {
              options.core.zmq.port = defaultConfig.getStored('core.zmq.port');
            }

            const configuredZmqPort = Number(options.core.zmq.port);
            if (isLocal && configuredZmqPort === 29998) {
              options.core.zmq.port = 49998;
            } else if (isTestnet && configuredZmqPort === 29998) {
              options.core.zmq.port = 39998;
            }

            if (!options.platform?.dapi) {
              return;
            }

            if (!options.platform.dapi.rsDapi) {
              options.platform.dapi.rsDapi = lodash.cloneDeep(defaultConfig.getStored('platform.dapi.rsDapi'));
            }

            const defaultMetrics = defaultConfig.getStored('platform.dapi.rsDapi.metrics');

            if (!options.platform.dapi.rsDapi.metrics) {
              options.platform.dapi.rsDapi.metrics = lodash.cloneDeep(defaultMetrics);
            }

            if (typeof options.platform.dapi.rsDapi.metrics.enabled === 'undefined') {
              options.platform.dapi.rsDapi.metrics.enabled = defaultMetrics.enabled;
            }

            if (typeof options.platform.dapi.rsDapi.metrics.port === 'undefined') {
              options.platform.dapi.rsDapi.metrics.port = defaultMetrics.port;
            }

            const configuredMetricsPort = Number(options.platform.dapi.rsDapi.metrics.port);
            if (isLocal && configuredMetricsPort === 9091) {
              options.platform.dapi.rsDapi.metrics.port = 29091;
            } else if (isTestnet && configuredMetricsPort === 9091) {
              options.platform.dapi.rsDapi.metrics.port = 19091;
            }

            if (options.platform.dapi.api) {
              const { waitForStResultTimeout } = options.platform.dapi.api;

              if (typeof waitForStResultTimeout === 'number'
                && typeof options.platform.dapi.rsDapi.waitForStResultTimeout === 'undefined') {
                options.platform.dapi.rsDapi.waitForStResultTimeout = waitForStResultTimeout;
              }

              delete options.platform.dapi.api;
            }

            if (typeof options.platform.dapi.rsDapi.waitForStResultTimeout === 'undefined') {
              options.platform.dapi.rsDapi.waitForStResultTimeout = defaultConfig.getStored('platform.dapi.rsDapi.waitForStResultTimeout');
            }

            if (options.platform?.dapi?.deprecated) {
              delete options.platform.dapi.deprecated;
            }

            // --- Gateway upstreams migration ---
            if (options.platform?.gateway?.upstreams) {
              const { upstreams } = options.platform.gateway;
              const defaultUpstreams = defaultConfig.getStored('platform.gateway.upstreams');

              if (!upstreams.rsDapi) {
                const { dapiApi, dapiCoreStreams } = upstreams;
                const dapiApiMax = dapiApi?.maxRequests;
                const dapiCoreStreamsMax = dapiCoreStreams?.maxRequests;

                const candidates = [
                  typeof dapiApiMax === 'number' ? dapiApiMax : null,
                  typeof dapiCoreStreamsMax === 'number' ? dapiCoreStreamsMax : null,
                ].filter((value) => value !== null);

                if (candidates.length > 0) {
                  upstreams.rsDapi = {
                    maxRequests: Math.max(...candidates),
                  };
                } else {
                  upstreams.rsDapi = lodash.cloneDeep(defaultUpstreams.rsDapi);
                }
              }

              delete upstreams.dapiApi;
              delete upstreams.dapiCoreStreams;
            }

            if (options.platform?.drive?.abci?.docker
              && defaultConfig.has('platform.drive.abci.docker.image')) {
              repinStockImage(options.platform?.drive?.abci?.docker, 'dashpay/drive', defaultConfig.getStored('platform.drive.abci.docker.image'));
            }

            if (options.platform.dapi?.rsDapi?.docker
              && defaultConfig.has('platform.dapi.rsDapi.docker.image')) {
              repinStockImage(options.platform?.dapi?.rsDapi?.docker, 'dashpay/rs-dapi', defaultConfig.getStored('platform.dapi.rsDapi.docker.image'));
            }

            if (!options.platform.quorumList) {
              options.platform.quorumList = lodash.cloneDeep(defaultConfig.getStored('platform.quorumList'));
            }

            if (!options.core.rpc.users.quorum_list) {
              options.core.rpc.users.quorum_list = lodash.cloneDeep(
                defaultConfig.getStored('core.rpc.users.quorum_list'),
              );
            }

            // --- Letsencrypt provider config ---
            if (options.platform?.gateway?.ssl?.providerConfigs
              && !options.platform.gateway.ssl.providerConfigs.letsencrypt) {
              options.platform.gateway.ssl.providerConfigs.letsencrypt = {
                email: null,
              };
            }
          });

        return configFile;
      },
      '3.0.2': (configFile) => {
        // Patch the Platform Gateway (Envoy) image for CVE-2026-47774 /
        // GHSA-22m2-hvr2-xqc8: an unauthenticated HTTP/2 downstream
        // memory-exhaustion DoS. Only configs still on the EOL,
        // dashmate-shipped 1.30.x Envoy image are bumped to the patched base
        // default (Envoy 1.35.11); a deliberately customised image (private
        // fork, vendor-patched build, `:latest`, etc.) is left untouched.
        const patchedImage = base.getStored('platform.gateway.docker.image');
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            const docker = options.platform?.gateway?.docker;
            if (docker && /^dashpay\/envoy:1\.30\./.test(docker.image)) {
              docker.image = patchedImage;
            }
          });

        return configFile;
      },
      '3.0.1': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.core.docker.image = 'dashpay/dashd:23';
          });

        return configFile;
      },
      '3.1.0': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            const defaultConfig = getDefaultConfigByNameOrGroup(name, options.group);
            const isLocal = options.network === NETWORK_LOCAL || name === 'local';
            const isTestnet = options.network === NETWORK_TESTNET || name === 'testnet';

            // Flip `core.compactFilters` to true for every config —
            // pre-3.1.0 configs predate the field entirely (template
            // emitted nothing, dashcore left the cfilter index off),
            // so a missing-or-false value here always means
            // "inherited the old implicit-off default" rather than
            // "user explicitly opted out". The base config now
            // ships with this flag on; this backfill brings every
            // already-set-up cluster up to that line so the iOS
            // BIP157 SPV flow against `local_seed` (and any other
            // dashmate node) works without manual editing.
            if (options.core) {
              options.core.compactFilters = true;
            }

            if (options.platform?.drive?.tenderdash?.docker
              && defaultConfig.has('platform.drive.tenderdash.docker.image')) {
              options.platform.drive.tenderdash.docker.image = defaultConfig
                .getStored('platform.drive.tenderdash.docker.image');
            }

            // Backfill the new `buildArgs: {}` field on each build block —
            // forwarded into `dynamic-compose.yml` as `build.args` entries.
            // Pre-3.1.0 configs predate the field; default it to an empty
            // object when missing (idempotent: existing values are preserved).
            if (options.platform?.drive?.abci?.docker?.build
              && typeof options.platform.drive.abci.docker.build.buildArgs === 'undefined') {
              options.platform.drive.abci.docker.build.buildArgs = {};
            }

            if (options.platform?.dapi?.rsDapi?.docker?.build
              && typeof options.platform.dapi.rsDapi.docker.build.buildArgs === 'undefined') {
              options.platform.dapi.rsDapi.docker.build.buildArgs = {};
            }

            if (options.platform?.drive?.tenderdash?.p2p
              && typeof options.platform.drive.tenderdash.p2p.allowlistOnly === 'undefined') {
              options.platform.drive.tenderdash.p2p.allowlistOnly = defaultConfig
                .getStored('platform.drive.tenderdash.p2p.allowlistOnly');
            }

            // --- Differentiate ports between networks to avoid conflicts ---
            // when running multiple networks on the same machine (issue #3002)
            // Note: earlier migrations may assign objects from base.getStored() without
            // cloning, causing shared references. We must clone before mutating.

            if (!isTestnet && !isLocal) {
              return;
            }

            const networkConfig = getDefaultConfigByNetwork(options.network);

            const portPaths = [
              'dashmate.helper.api',
              'core.insight',
            ];

            for (const parentPath of portPaths) {
              const obj = lodash.get(options, parentPath);
              if (obj && Number(obj.port) === base.getStored(`${parentPath}.port`)) {
                lodash.set(options, parentPath, lodash.cloneDeep(obj));
                lodash.get(options, parentPath).port = networkConfig.getStored(`${parentPath}.port`);
              }
            }

            if (!options.platform) {
              return;
            }

            const platformPortPaths = [
              'platform.gateway.metrics',
              'platform.gateway.admin',
              'platform.gateway.rateLimiter.metrics',
              'platform.quorumList.api',
              'platform.drive.abci.tokioConsole',
              'platform.drive.abci.metrics',
              'platform.drive.abci.grovedbVisualizer',
            ];

            for (const parentPath of platformPortPaths) {
              const obj = lodash.get(options, parentPath);
              if (obj && Number(obj.port) === base.getStored(`${parentPath}.port`)) {
                lodash.set(options, parentPath, lodash.cloneDeep(obj));
                lodash.get(options, parentPath).port = networkConfig.getStored(`${parentPath}.port`);
              }
            }
          });

        return configFile;
      },
      '4.0.0-rc.3': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            // Bump the default Tenderdash image to the 1.6.0 line. Pulled DRY from
            // the base config so it tracks whatever the base config pins.
            // Keyed at the next release (4.0.0-rc.3), not the already-released
            // rc.2: the runner skips fromVersion===toVersion, so a key equal to
            // an operator's current version never fires.
            options.platform.drive.tenderdash.docker.image = base.getStored('platform.drive.tenderdash.docker.image');

            // Add responseHeaders toggle to rate limiter (default true so existing
            // deployments keep emitting RateLimit-* headers; rs-dapi-client depends
            // on RateLimit-Reset to apply precise ban windows instead of the
            // exponential health-ban ladder).
            // Keyed at the next release (4.0.0-rc.3), not the already-released
            // rc.2: the runner skips fromVersion===toVersion, so a key equal to
            // an operator's current version never fires. Backfill runs once the
            // package bumps to rc.3 (mirrors the 3.1.0 migration added at 3.1.0-dev.1).
            if (options.platform?.gateway?.rateLimiter
              && typeof options.platform.gateway.rateLimiter.responseHeaders === 'undefined') {
              options.platform.gateway.rateLimiter.responseHeaders = base.getStored('platform.gateway.rateLimiter.responseHeaders');
            }
          });

        return configFile;
      },
      '4.0.0': (configFile) => {
        // The drive and rs-dapi image tags are derived from the package major
        // version. Re-pin them from the base config so operators upgrading from
        // a prerelease of this major, or from an older major, move off their
        // stale tag. The legacy 0.25.x migrations already do this, but only fire
        // for configs old enough to cross them; recent upgraders need it here.
        //
        // Only tags a release published are moved. This re-pin used to be
        // unconditional, which destroyed an operator's own image before any
        // later migration could tell it apart from a stale default - every
        // config from before this key crosses here, so it has to be the place
        // that distinction is first respected. Tags of every era are recognised
        // because a config reaching this point may carry any of them.
        const stockDriveImage = historicalStockImagePattern('dashpay/drive');
        const stockRsDapiImage = historicalStockImagePattern('dashpay/rs-dapi');

        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            const driveDocker = options.platform?.drive?.abci?.docker;
            if (driveDocker && stockDriveImage.test(driveDocker.image)) {
              driveDocker.image = base.getStored('platform.drive.abci.docker.image');
            }

            const rsDapiDocker = options.platform?.dapi?.rsDapi?.docker;
            if (rsDapiDocker && stockRsDapiImage.test(rsDapiDocker.image)) {
              rsDapiDocker.image = base.getStored('platform.dapi.rsDapi.docker.image');
            }
          });

        return configFile;
      },
      '4.1.0-rc.2': (configFile) => {
        // Move the Platform Gateway onto the Envoy 1.39 line. Only configs still
        // carrying the previously shipped 1.35.x image are re-pinned; an image
        // the operator chose themselves (private fork, vendor-patched build,
        // floating tag) is left alone. Pulled from the base config so it tracks
        // whatever is pinned there.
        // Keyed at the next release, not the released 4.1.0-rc.1: the runner
        // skips fromVersion===toVersion, so a key equal to an operator's current
        // version never fires.
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            const docker = options.platform?.gateway?.docker;
            if (docker && /^dashpay\/envoy:1\.35\./.test(docker.image)) {
              docker.image = base.getStored('platform.gateway.docker.image');
            }
          });

        return configFile;
      },
      '4.1.0-rc.3': (configFile) => {
        // The drive and rs-dapi image tags are derived from the package version
        // in configs/defaults/getBaseConfigFactory.js, so operators upgrading
        // from an earlier release of this major keep pulling the images of the
        // line they installed until the tags are re-pinned from the base config.
        // Keyed one release ahead: the runner skips fromVersion === toVersion,
        // so a migration keyed at an operator's current version never fires.
        //
        // Only tags a release published are moved, so a tag the operator chose
        // in this namespace (dashpay/drive:4-local) is left alone. The major is
        // the one being migrated away from and stays 4; a later major needs its
        // own migration.
        const stockDriveImage = stockImagePattern('dashpay/drive', 4);
        const stockRsDapiImage = stockImagePattern('dashpay/rs-dapi', 4);

        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            const driveDocker = options.platform?.drive?.abci?.docker;
            if (driveDocker && stockDriveImage.test(driveDocker.image)) {
              driveDocker.image = base.getStored('platform.drive.abci.docker.image');
            }

            const rsDapiDocker = options.platform?.dapi?.rsDapi?.docker;
            if (rsDapiDocker && stockRsDapiImage.test(rsDapiDocker.image)) {
              rsDapiDocker.image = base.getStored('platform.dapi.rsDapi.docker.image');
            }
          });

        return configFile;
      },
      '4.1.0-rc.4': (configFile) => {
        // Stop storing the version-derived image tags. A config now records
        // whether the operator chose an image, not which image a past release
        // happened to derive: null means "use the line this dashmate build
        // ships", and a string is the operator's own and is never touched.
        //
        // This is the last time a stock tag has to be recognised by shape. From
        // here on the distinction is recorded rather than inferred, so no future
        // release needs a migration to re-pin these images.
        const stockDriveImage = stockImagePattern('dashpay/drive', 4);
        const stockRsDapiImage = stockImagePattern('dashpay/rs-dapi', 4);

        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            const driveDocker = options.platform?.drive?.abci?.docker;
            if (driveDocker && stockDriveImage.test(driveDocker.image)) {
              driveDocker.image = null;
            }

            const rsDapiDocker = options.platform?.dapi?.rsDapi?.docker;
            if (rsDapiDocker && stockRsDapiImage.test(rsDapiDocker.image)) {
              rsDapiDocker.image = null;
            }
          });

        return configFile;
      },
      '4.1.0': (configFile) => {
        // Counterpart of the release-candidate migration for the stable release.
        // An operator who ran a 4.1 release candidate carries a `-rc` image tag,
        // and the migration that set it no longer fires once they are on the rc
        // line. Re-pin the drive and rs-dapi tags from the base config so a
        // stable upgrade moves them off `4-rc` onto the stable `4` images.
        //
        // Only tags a release published are moved, so a tag the operator chose
        // in this namespace (dashpay/drive:4-local) is left alone.
        const stockDriveImage = stockImagePattern('dashpay/drive', 4);
        const stockRsDapiImage = stockImagePattern('dashpay/rs-dapi', 4);

        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            const driveDocker = options.platform?.drive?.abci?.docker;
            if (driveDocker && stockDriveImage.test(driveDocker.image)) {
              driveDocker.image = base.get('platform.drive.abci.docker.image');
            }

            const rsDapiDocker = options.platform?.dapi?.rsDapi?.docker;
            if (rsDapiDocker && stockRsDapiImage.test(rsDapiDocker.image)) {
              rsDapiDocker.image = base.get('platform.dapi.rsDapi.docker.image');
            }
          });

        return configFile;
      },
      '4.2.0': (configFile) => {
        // The ACME directory certificates are requested from became
        // configurable. Existing configs have no value for it, and the schema
        // requires one, so fill in the directory they were already using.
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            const providerConfigs = options.platform?.gateway?.ssl?.providerConfigs;

            if (providerConfigs?.letsencrypt
              && providerConfigs.letsencrypt.acmeDirectoryUrl === undefined) {
              providerConfigs.letsencrypt.acmeDirectoryUrl = base.get(
                'platform.gateway.ssl.providerConfigs.letsencrypt.acmeDirectoryUrl',
              );
            }
          });

        return configFile;
      },
      '4.1.1': (configFile) => {
        // The drive and rs-dapi tags are derived from the package version, and
        // the migration that re-pins them no longer fires for a config already
        // stamped at the release that set it. This release is the next one above
        // that stamp, so it carries the re-pin forward for a config that never
        // crossed it.
        //
        // Only tags a release published are moved, so a tag the operator chose
        // in this namespace (dashpay/drive:4-local) is left alone.
        const stockDriveImage = stockImagePattern('dashpay/drive', 4);
        const stockRsDapiImage = stockImagePattern('dashpay/rs-dapi', 4);

        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            // Move the Tenderdash image onto the floating minor tag the base
            // config now pins, so a patch release published on that line is
            // picked up without a Dashmate release. Configs written while the
            // base config pinned an exact version hold that version, and only a
            // migration moves them off it. Pulled from the base config so it
            // tracks whatever is pinned there.
            // Keyed at the next release, not the released 4.1.0: the runner
            // skips fromVersion===toVersion, so a key equal to an operator's
            // current version never fires.
            if (options.platform?.drive?.tenderdash?.docker) {
              options.platform.drive.tenderdash.docker.image = base.get('platform.drive.tenderdash.docker.image');
            }

            const driveDocker = options.platform?.drive?.abci?.docker;
            if (driveDocker && stockDriveImage.test(driveDocker.image)) {
              driveDocker.image = base.get('platform.drive.abci.docker.image');
            }

            const rsDapiDocker = options.platform?.dapi?.rsDapi?.docker;
            if (rsDapiDocker && stockRsDapiImage.test(rsDapiDocker.image)) {
              rsDapiDocker.image = base.get('platform.dapi.rsDapi.docker.image');
            }

            // The Commit timeout and BypassCommitTimeout overrides no longer
            // exist in Tenderdash, which now only warns when they are set.
            // Drop them: the config schema accepts no properties it does not
            // define, so a config that kept them would fail validation.
            delete options.platform?.drive?.tenderdash?.consensus?.unsafeOverride?.commit;
          });

        return configFile;
      },
    };
  }

  return getConfigFileMigrations;
}
