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
              host: '34.209.100.240',
              port: 26656,
            },
            {
              id: '2006632eb20e670923d13d4f53abc24468eaad4d',
              host: '54.213.254.17',
              port: 26656,
            },
          ],
        },
        genesis: {
          genesis_time: '2021-07-22T12:57:05.429Z',
          chain_id: 'dash-testnet-16',
          initial_height: '0',
          initial_core_chain_locked_height: 801340,
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
            version: {},
          },
          validators: [
            {
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value: 'imxjukh5hRY91Mvm/sfhQp6iSnICyvKMMdhY5Sq6Ej0QJyB3vtN4UfYwvmxdzOVM',
              },
              power: '100',
              name: '',
              pro_tx_hash: 'F3D506822A24E7E4BE318A6ED7371CC1E1527880A594FE04629F50A1618DB8E7',
            },
          ],
          threshold_public_key: {
            type: 'tendermint/PubKeyBLS12381',
            value: 'imxjukh5hRY91Mvm/sfhQp6iSnICyvKMMdhY5Sq6Ej0QJyB3vtN4UfYwvmxdzOVM',
          },
          quorum_type: '4',
          quorum_hash: '0000000000000000000000000000000000000000',
          app_hash: '',
        },
      },
    },
    dpns: {
      masterPublicKey: '038b74aea104c19463b74be5fae9af2255fe42013aecd17092464214f2867ac19b',
      secondPublicKey: '029df453a626cde501f454b44a9beeb8e525590c94ef2e78743993006651fcec4f',
    },
    dashpay: {
      masterPublicKey: '02e57fb84728e6f10c885cf4d17025f8c6d7016321e4fab7b7ca8135d6e06b5ec2',
      secondPublicKey: '03c7024f5f375f9a04152c8438e9bca612d084c752269c28965146caa73e223118',
    },
    featureFlags: {
      masterPublicKey: '03aff4e943e005a2549757fdd308b0a7a04b03d74d78d53f981dd337ace47994cc',
      secondPublicKey: '0311c23cf3ba6cdb32834d5a1ce2e4258ff7766a8e977f6318063d0bfe9bd476c3',
    },
    masternodeRewardShares: {
      masterPublicKey: '0211234327aed200b2771788aec1c7d6a799f534d02dd6766c6de53e3fd7152dfc',
      secondPublicKey: '035655d53d061275314535b74bfbbfb74cf640023a5cc466283e83881876cb9a3f',
    },
  },
  network: NETWORK_TESTNET,
});
