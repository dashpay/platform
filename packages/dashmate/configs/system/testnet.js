const lodashMerge = require('lodash.merge');
const os = require('os');
const path = require('path');

const {
  NETWORK_TESTNET,
} = require('../../src/constants');

const baseConfig = require('./base');

module.exports = lodashMerge({}, baseConfig, {
  description: 'node with testnet configuration',
  core: {
    p2p: {
      port: 19999,
    },
    rpc: {
      port: 19998,
    },
  },
  platform: {
    dpns: {
      contract: {
        id: '6Atad7WNCeXAnavaKTRkpS2dipgVtfWFa67YmYFVgxNA',
        blockHeight: 17,
      },
      ownerId: 'EfzY3B3XjMJP3YtGtjyT1rMK7tTiuBCbsDqE9NwUzyGb',
    },
    dashpay: {
      contract: {
        id: 'D1SaMNTArb9mM7XiKsubdLyC5UsMDnbkwiUhrHEPXxm2',
        blockHeight: 28,
      },
    },
    featureFlags: {
      contract: {
        id: '3imKEmir6daXHoAQ8FUa8MQ9KqQFMZArDySVqZb32f5N',
        blockHeight: 34,
      },
      ownerId: '471GAr41D7eY7veU1X66Rk8jfa6jSo3dr6krCpo6YvXh',
    },
    drive: {
      abci: {
        log: {
          prettyFile: {
            path: path.join(os.tmpdir(), '/testnet-drive-pretty.log'),
          },
          jsonFile: {
            path: path.join(os.tmpdir(), '/testnet-drive-json.log'),
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
          chain_id: 'dash-testnet-3',
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
              pub_key_types: ['bls12381'],
            },
            version: {},
          },
          threshold_public_key: null,
          quorum_type: '4',
          quorum_hash: null,
          app_hash: '',
        },
      },
    },
  },
  network: NETWORK_TESTNET,
});
