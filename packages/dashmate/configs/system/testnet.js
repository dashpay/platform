const lodashMerge = require('lodash.merge');
const path = require('path');

const {
  NETWORK_TESTNET,
  HOME_DIR_PATH,
} = require('../../src/constants');

const baseConfig = require('./base');

module.exports = lodashMerge({}, baseConfig, {
  description: 'node with testnet configuration',
  docker: {
    network: {
      subnet: '172.25.24.0/24',
    },
  },
  core: {
    p2p: {
      port: 19999,
    },
    rpc: {
      port: 19998,
    },
  },
  platform: {
    drive: {
      abci: {
        log: {
          prettyFile: {
            path: path.join(HOME_DIR_PATH, 'logs', 'testnet', 'drive-pretty.log'),
          },
          jsonFile: {
            path: path.join(HOME_DIR_PATH, 'logs', 'testnet', 'drive-json.log'),
          },
        },
      },
      tenderdash: {
        p2p: {
          seeds: [
            {
              id: '74907790a03b51ac062c8a1453dafd72a08668a3',
              host: '54.189.200.56',
              port: 26656,
            },
            {
              id: '2006632eb20e670923d13d4f53abc24468eaad4d',
              host: '52.43.162.96',
              port: 26656,
            },
          ],
        },
        genesis: {
          genesis_time: '2021-07-22T12:57:05.429Z',
          chain_id: 'dash-testnet-8',
          initial_height: '0',
          initial_core_chain_locked_height: 542300,
          initial_proposal_core_chain_lock: null,
          consensus_params: {
            block: {
              max_bytes: '22020096',
              max_gas: '-1',
              time_iota_ms: '5000',
            },
            evidence: {
              max_age: '100000',
              max_age_num_blocks: '100000',
              max_age_duration: '172800000000000',
            },
            validator: {
              pub_key_types: [
                'bls12381',
              ],
            },
            version: {
              app_version: '1',
            },
            synchrony: {
              precision: '505000000',
              message_delay: '12000000000',
            },
            timeout: {
              propose: '3000000000',
              propose_delta: '500000000',
              vote: '1000000000',
              vote_delta: '500000000',
              commit: '1000000000',
              bypass_commit_timeout: false,
            },
          },
          threshold_public_key: null,
          quorum_type: '4',
          quorum_hash: null,
          app_hash: '',
        },
      },
    },
    dpns: {
      masterPublicKey: '022a5ffc9f92e005a02401c375f575b3aed5606fb24ddef5b3a05d55c66ba2a2f6',
    },
    dashpay: {
      masterPublicKey: '02c6bf10f8cc078866ed5466a0b5ea3a4e8db2a764ea5aa9cb75f22658664eb149',
    },
    featureFlags: {
      masterPublicKey: '033d57d03ba602acecfb6fd4ad66c5fdb9a739e163faefa901926bdf28063f9251',
    },
    masternodeRewardShares: {
      masterPublicKey: '02182c19827a5e3151feb965b2c6e6bbe57bb1f2fe7579595d76b672966da4e8e6',
    },
  },
  network: NETWORK_TESTNET,
});
