import lodash from 'lodash';

import {
  NETWORK_LOCAL, SSL_PROVIDERS,
} from '../../src/constants.js';
import Config from '../../src/config/Config.js';

const { merge: lodashMerge } = lodash;
/**
 * @param {getBaseConfig} getBaseConfig
 * @returns {getLocalConfig}
 */
export default function getLocalConfigFactory(getBaseConfig) {
  /**
   * @typedef {function} getLocalConfig
   * @returns {Config}
   */
  function getLocalConfig() {
    const options = {
      description: 'template for local configs',
      docker: {
        network: {
          subnet: '172.24.24.0/24',
        },
      },
      core: {
        docker: {
          image: 'dashpay/dashd:21.0.0-devpr6115.db828177',
          commandArgs: [],
        },
        p2p: {
          port: 20001,
        },
        rpc: {
          port: 20002,
        },
      },
      platform: {
        gateway: {
          ssl: {
            provider: SSL_PROVIDERS.SELF_SIGNED,
          },
          listeners: {
            dapiAndDrive: {
              port: 2443,
            },
          },
          rateLimiter: {
            enabled: false,
          },
        },
        drive: {
          tenderdash: {
            p2p: {
              port: 46656,
              flushThrottleTimeout: '10ms',
              maxPacketMsgPayloadSize: 1024,
              sendRate: 20000000,
              recvRate: 20000000,
            },
            rpc: {
              port: 46657,
            },
            pprof: {
              port: 46060,
            },
            metrics: {
              port: 46660,
            },
          },
          abci: {
            validatorSet: {
              quorum: {
                llmqType: 106,
                dkgInterval: 24,
                activeSigners: 2,
                rotation: false,
              },
            },
            chainLock: {
              quorum: {
                llmqType: 100,
                dkgInterval: 24,
                activeSigners: 2,
                rotation: false,
              },
            },
            instantLock: {
              quorum: {
                llmqType: 104,
                dkgInterval: 24,
                activeSigners: 2,
                rotation: false,
              },
            },
          },
        },
      },
      environment: 'development',
      network: NETWORK_LOCAL,
    };

    return new Config('local', lodashMerge({}, getBaseConfig()
      .getOptions(), options));
  }

  return getLocalConfig;
}
