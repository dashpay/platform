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
            options.core.docker.image = base.get('core.docker.image');

            options.core.sentinel.docker.image = base.get('core.sentinel.docker.image');

            options.dashmate.helper.docker.image = base.get('dashmate.helper.docker.image');

            options.platform.drive.tenderdash.docker.image = base.get('platform.drive.tenderdash.docker.image');

            options.platform.drive.abci.docker.image = base.get('platform.drive.abci.docker.image');

            options.platform.dapi.api.docker.image = base.get('platform.dapi.api.docker.image');

            options.platform.gateway.docker.image = base.get('platform.gateway.docker.image');
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
            options.platform.gateway.docker = base.get('platform.gateway.docker');

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
              options.platform.drive.tenderdash.metrics = defaultConfigs.get(name)
                .get('platform.drive.tenderdash.metrics');
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
              options.platform.drive.tenderdash.genesis.genesis_time = '2024-07-17T17:15:00.000Z';
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
              options.platform.drive.tenderdash.genesis.genesis_time = '2024-07-17T17:15:00.000Z';
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
          .forEach(([name, options]) => {
            if (options.network === NETWORK_TESTNET && name !== 'base') {
              options.platform.drive.abci.epochTime = testnet.get('platform.drive.abci.epochTime');
            }
            options.platform.drive.abci.docker.image = base.get('platform.drive.abci.docker.image');
            options.platform.dapi.api.docker.image = base.get('platform.dapi.api.docker.image');
          });

        return configFile;
      },
      '0.25.4': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            delete options.platform.drive.abci.log;

            options.platform.drive.abci.logs = base.get('platform.drive.abci.logs');
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
          configFile.configs.base.core.docker.image = base.get('core.docker.image');
        }
        if (configFile.configs.local) {
          configFile.configs.local.platform.dapi.envoy.ssl.provider = SSL_PROVIDERS.SELF_SIGNED;
        }

        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.drive.tenderdash.log.level = 'info';

            if (options.network !== NETWORK_MAINNET && options.network !== NETWORK_TESTNET) {
              options.core.docker.image = base.get('core.docker.image');
            }

            options.core.docker.commandArgs = [];
          });

        return configFile;
      },
      '0.25.12': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            options.platform.drive.tenderdash.docker.image = base.get('platform.drive.tenderdash.docker.image');

            if (options.network === NETWORK_TESTNET) {
              options.core.docker.image = base.get('core.docker.image');

              if (name !== base.getName()) {
                options.platform.drive.tenderdash.genesis.chain_id = testnet.get('platform.drive.tenderdash.genesis.chain_id');
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
            options.core.insight = base.get('core.insight');
            options.core.docker.image = base.get('core.docker.image');

            if (options.network === NETWORK_TESTNET && name !== base.getName()) {
              options.platform.drive.tenderdash.genesis.chain_id = testnet.get('platform.drive.tenderdash.genesis.chain_id');
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
              options.platform.drive.tenderdash.genesis.chain_id = testnet.get('platform.drive.tenderdash.genesis.chain_id');
              options.platform.drive.tenderdash.genesis.initial_core_chain_locked_height = 1400;
              options.platform.drive.tenderdash.genesis.genesis_time = '2024-07-17T17:15:00.000Z';
            }
          });

        return configFile;
      },
      '0.25.16-rc.6': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.core.docker.image = base.get('core.docker.image');
          });

        return configFile;
      },
      '0.25.16-rc.7': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.drive.tenderdash.docker.image = base.get('platform.drive.tenderdash.docker.image');

            delete options.docker.network.bindIp;

            options.core.p2p.host = base.get('core.p2p.host');
            options.core.rpc.host = base.get('core.rpc.host');
            options.platform.dapi.envoy.http.host = '0.0.0.0';
            options.platform.drive.tenderdash.p2p.host = base.get('platform.drive.tenderdash.p2p.host');
            options.platform.drive.tenderdash.rpc.host = base.get('platform.drive.tenderdash.rpc.host');
            options.platform.drive.tenderdash.metrics.host = base.get('platform.drive.tenderdash.metrics.host');
          });

        return configFile;
      },
      '0.25.19': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.drive.tenderdash.docker.image = base.get('platform.drive.tenderdash.docker.image');
          });

        return configFile;
      },
      '0.25.20': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            options.platform.dapi.envoy.http.connectTimeout = '5s';
            options.platform.dapi.envoy.http.responseTimeout = '15s';

            options.platform.drive.tenderdash.rpc.maxOpenConnections = base.get('platform.drive.tenderdash.rpc.maxOpenConnections');

            let defaultConfigName = 'base';
            if (options.group === 'local' || name === 'local') {
              defaultConfigName = 'local';
            }
            const defaultConfig = defaultConfigs.get(defaultConfigName);

            options.platform.drive.tenderdash.p2p.flushThrottleTimeout = defaultConfig.get('platform.drive.tenderdash.p2p.flushThrottleTimeout');
            options.platform.drive.tenderdash.p2p.maxPacketMsgPayloadSize = defaultConfig.get('platform.drive.tenderdash.p2p.maxPacketMsgPayloadSize');
            options.platform.drive.tenderdash.p2p.sendRate = defaultConfig.get('platform.drive.tenderdash.p2p.sendRate');
            options.platform.drive.tenderdash.p2p.recvRate = defaultConfig.get('platform.drive.tenderdash.p2p.recvRate');

            options.platform.drive.tenderdash.mempool = lodash.clone(base.get('platform.drive.tenderdash.mempool'));
            options.platform.drive.tenderdash.consensus.peer = base.get('platform.drive.tenderdash.consensus.peer');
            options.platform.drive.tenderdash.consensus.unsafeOverride = base.get('platform.drive.tenderdash.consensus.unsafeOverride');
          });

        return configFile;
      },
      '1.0.0-dev.2': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            if (defaultConfigs.has(name)) {
              options.platform.drive.tenderdash.genesis = defaultConfigs.get(name)
                .get('platform.drive.tenderdash.genesis');
            }
            options.platform.dapi.api.docker.deploy = base.get('platform.dapi.api.docker.deploy');

            let baseConfigName = name;
            if (options.group !== null && defaultConfigs.has(options.group)) {
              baseConfigName = options.group;
            } else if (!defaultConfigs.has(baseConfigName)) {
              baseConfigName = 'testnet';
            }

            options.platform.drive.abci.chainLock = defaultConfigs.get(baseConfigName)
              .get('platform.drive.abci.chainLock');
          });

        return configFile;
      },
      '1.0.0-dev.4': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            const defaultConfig = getDefaultConfigByNameOrGroup(name, options.group);
            options.core.docker.image = defaultConfig.get('core.docker.image');

            options.platform.drive.tenderdash.docker.image = defaultConfig.get('platform.drive.tenderdash.docker.image');
          });

        return configFile;
      },
      '1.0.0-dev.5': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.drive.tenderdash.mempool.cacheSize = base.get('platform.drive.tenderdash.mempool.cacheSize');
          });

        return configFile;
      },
      '1.0.0-dev.6': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            options.platform.drive.abci.tokioConsole = base.get('platform.drive.abci.tokioConsole');

            const defaultConfig = getDefaultConfigByNameOrGroup(name, options.group);
            options.platform.drive.tenderdash.docker.image = defaultConfig.get('platform.drive.tenderdash.docker.image');
          });

        return configFile;
      },
      '1.0.0-dev.7': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            if (options.network === NETWORK_TESTNET && name !== 'base') {
              options.platform.drive.tenderdash.genesis = testnet.get('platform.drive.tenderdash.genesis');
            }

            const defaultConfig = getDefaultConfigByNameOrGroup(name, options.group);
            options.core.docker.image = defaultConfig.get('core.docker.image');
          });

        return configFile;
      },
      '1.0.0-dev.8': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            const defaultConfig = getDefaultConfigByNameOrGroup(name, options.group);
            options.core.docker.image = defaultConfig.get('core.docker.image');
          });

        return configFile;
      },
      '1.0.0-dev.9': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            options.platform.drive.tenderdash.docker.image = base.get('platform.drive.tenderdash.docker.image');

            const defaultConfig = getDefaultConfigByNameOrGroup(name, options.group);
            options.platform.drive.tenderdash.mempool.timeoutCheckTx = defaultConfig.get('platform.drive.tenderdash.mempool.timeoutCheckTx');
            options.platform.drive.tenderdash.mempool.txEnqueueTimeout = defaultConfig.get('platform.drive.tenderdash.mempool.txEnqueueTimeout');
            options.platform.drive.tenderdash.mempool.txSendRateLimit = defaultConfig.get('platform.drive.tenderdash.mempool.txSendRateLimit');
            options.platform.drive.tenderdash.mempool.txRecvRateLimit = defaultConfig.get('platform.drive.tenderdash.mempool.txRecvRateLimit');
            options.platform.drive.tenderdash.rpc.timeoutBroadcastTx = defaultConfig.get('platform.drive.tenderdash.rpc.timeoutBroadcastTx');
          });

        return configFile;
      },
      '1.0.0-dev.10': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.platform.drive.tenderdash.docker.image = base.get('platform.drive.tenderdash.docker.image');
          });

        return configFile;
      },
      '1.0.0-dev.12': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            // Update tenderdash config
            options.platform.drive.tenderdash.docker.image = base.get('platform.drive.tenderdash.docker.image');
            options.platform.drive.tenderdash.mempool.maxConcurrentCheckTx = base.get('platform.drive.tenderdash.mempool.maxConcurrentCheckTx');

            // Add metrics to Drive ABCI
            options.platform.drive.abci.metrics = base.get('platform.drive.abci.metrics');

            // Envoy -> Gateway
            if (options.platform.dapi.envoy) {
              options.platform.gateway = lodash.cloneDeep(options.platform.dapi.envoy);

              // add new options
              options.platform.gateway.maxConnections = base.get('platform.gateway.maxConnections');
              options.platform.gateway.maxHeapSizeInBytes = base.get('platform.gateway.maxHeapSizeInBytes');
              options.platform.gateway.metrics = base.get('platform.gateway.metrics');
              options.platform.gateway.admin = base.get('platform.gateway.admin');
              options.platform.gateway.upstreams = base.get('platform.gateway.upstreams');
              options.platform.gateway.log = base.get('platform.gateway.log');

              // http -> listeners
              options.platform.gateway.listeners = lodash.cloneDeep(
                base.get('platform.gateway.listeners'),
              );

              options.platform.gateway.listeners.dapiAndDrive.host = options.platform.dapi.envoy
                .http.host;
              options.platform.gateway.listeners.dapiAndDrive.port = options.platform.dapi.envoy
                .http.port;

              delete options.platform.gateway.http;

              // update rate limiter
              options.platform.gateway.rateLimiter.docker = base.get('platform.gateway.rateLimiter.docker');
              options.platform.gateway.rateLimiter.unit = base.get('platform.gateway.rateLimiter.unit');
              options.platform.gateway.rateLimiter.requestsPerUnit = base.get('platform.gateway.rateLimiter.requestsPerUnit');
              options.platform.gateway.rateLimiter.blacklist = base.get('platform.gateway.rateLimiter.blacklist');
              options.platform.gateway.rateLimiter.whitelist = base.get('platform.gateway.rateLimiter.whitelist');
              options.platform.gateway.rateLimiter.metrics = base.get('platform.gateway.rateLimiter.metrics');

              delete options.platform.gateway.rateLimiter.fillInterval;
              delete options.platform.gateway.rateLimiter.maxTokens;
              delete options.platform.gateway.rateLimiter.tokensPerFill;

              // delete envoy
              delete options.platform.dapi.envoy;

              // update image
              options.platform.gateway.docker.image = base.get('platform.gateway.docker.image');
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
              llmqType: networkConfig.get('platform.drive.abci.chainLock.quorum.llmqType'),
              dkgInterval: networkConfig.get('platform.drive.abci.chainLock.quorum.dkgInterval'),
              activeSigners: networkConfig.get('platform.drive.abci.chainLock.quorum.activeSigners'),
              rotation: networkConfig.get('platform.drive.abci.chainLock.quorum.rotation'),
            };

            delete options.platform.drive.abci.chainLock.llmqType;
            delete options.platform.drive.abci.chainLock.llmqSize;
            delete options.platform.drive.abci.chainLock.dkgInterval;

            options.platform.drive.abci.validatorSet.quorum = {
              llmqType: networkConfig.get('platform.drive.abci.validatorSet.quorum.llmqType'),
              dkgInterval: networkConfig.get('platform.drive.abci.validatorSet.quorum.dkgInterval'),
              activeSigners: networkConfig.get('platform.drive.abci.validatorSet.quorum.activeSigners'),
              rotation: networkConfig.get('platform.drive.abci.validatorSet.quorum.rotation'),
            };

            delete options.platform.drive.abci.validatorSet.llmqType;

            options.platform.drive.abci.instantLock = {
              quorum: {
                llmqType: networkConfig.get('platform.drive.abci.instantLock.quorum.llmqType'),
                dkgInterval: networkConfig.get('platform.drive.abci.instantLock.quorum.dkgInterval'),
                activeSigners: networkConfig.get('platform.drive.abci.instantLock.quorum.activeSigners'),
                rotation: networkConfig.get('platform.drive.abci.instantLock.quorum.rotation'),
              },
            };
          });

        return configFile;
      },
      '1.0.0-dev.17': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            options.platform.drive.tenderdash.docker.image = base.get('platform.drive.tenderdash.docker.image');
            options.platform.drive.abci.grovedbVisualizer = base.get('platform.drive.abci.grovedbVisualizer');

            // Update Core image
            options.core.docker.image = getDefaultConfigByNameOrGroup(name, options.group)
              .get('core.docker.image');

            // Update Core RPC auth configuration
            options.core.rpc.users = base.get('core.rpc.users');
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
              .get('core.docker.image');

            options.core.devnet.llmq = base.get('core.devnet.llmq');

            if (options.network === NETWORK_TESTNET) {
              options.platform.drive.tenderdash.genesis = testnet.get('platform.drive.tenderdash.genesis');
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
            options.platform.drive.tenderdash.docker.image = base.get('platform.drive.tenderdash.docker.image');

            // Replace quorumsign with qurumplatformsign in Core RPC Tenderdash auth whitelist
            options.core.rpc.users.tenderdash.whitelist = base.get('core.rpc.users.tenderdash.whitelist');
          });
        return configFile;
      },
      '1.0.0-rc.2': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            if (options.network === NETWORK_TESTNET) {
              options.platform.drive.tenderdash.genesis = testnet.get('platform.drive.tenderdash.genesis');
            }

            // Update tenderdash image
            options.platform.drive.tenderdash.docker.image = base.get('platform.drive.tenderdash.docker.image');
            options.core.rpc.users.drive_consensus.whitelist = base.get('core.rpc.users.drive_consensus.whitelist');
          });
        return configFile;
      },
      '1.0.0': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([name, options]) => {
            if (name === 'base') {
              options.platform.drive.tenderdash.mempool = base.get('platform.drive.tenderdash.mempool');
              options.platform.drive.tenderdash.genesis = base.get('platform.drive.tenderdash.genesis');
            } else if (options.network === NETWORK_MAINNET) {
              options.platform.drive.tenderdash.p2p = mainnet.get('platform.drive.tenderdash.p2p');
              options.platform.drive.tenderdash.mempool = mainnet.get('platform.drive.tenderdash.mempool');
              options.platform.drive.tenderdash.genesis = mainnet.get('platform.drive.tenderdash.genesis');

              if (options.platform.drive.tenderdash.node.id !== null) {
                options.platform.enable = true;
              }
            }

            // Update tenderdash image
            options.platform.drive.tenderdash.docker.image = base.get('platform.drive.tenderdash.docker.image');
            options.core.docker.image = base.get('core.docker.image');
          });
        return configFile;
      },
      '1.0.2': (configFile) => {
        Object.entries(configFile.configs)
          .forEach(([, options]) => {
            options.core.indexes = [];
            options.platform.drive.abci.docker.image = 'dashpay/drive:1';
            options.platform.dapi.api.docker.image = 'dashpay/dapi:1';
          });
        return configFile;
      },
    };
  }

  return getConfigFileMigrations;
}
