const lodashMerge = require('lodash.merge');

const NETWORKS = require('../../networks');

const baseConfig = {
  description: 'base config for use as template',
  core: {
    docker: {
      image: 'dashpay/dashd:0.17.0.0-rc3-hotfix1',
    },
    p2p: {
      port: 20001,
      seeds: [],
    },
    rpc: {
      port: 20002,
      user: 'dashrpc',
      password: 'rpcpassword',
    },
    spork: {
      address: null,
      privateKey: null,
    },
    masternode: {
      enable: true,
      operator: {
        privateKey: null,
      },
    },
    miner: {
      enable: false,
      interval: '2.5m',
      address: null,
    },
    sentinel: {
      docker: {
        image: 'dashpay/sentinel:1.5.0',
      },
    },
    devnetName: null,
  },
  platform: {
    dapi: {
      envoy: {
        docker: {
          image: 'envoyproxy/envoy:v1.16-latest',
        },
      },
      nginx: {
        http: {
          port: 3000,
        },
        grpc: {
          port: 3010,
        },
        docker: {
          image: 'nginx:latest',
        },
        rateLimiter: {
          enable: true,
          rate: 120,
          burst: 300,
        },
      },
      api: {
        docker: {
          image: 'dashpay/dapi:0.18-dev',
        },
      },
      insight: {
        docker: {
          image: 'dashpay/insight-api:3.1.1',
        },
      },
    },
    drive: {
      mongodb: {
        docker: {
          image: 'mongo:4.2',
        },
      },
      abci: {
        docker: {
          image: 'dashpay/drive:0.18-dev',
        },
        log: {
          stdout: {
            level: 'info',
          },
          prettyFile: {
            level: 'silent',
            path: '/tmp/base-drive-pretty.log',
          },
          jsonFile: {
            level: 'silent',
            path: '/tmp/base-drive-json.json',
          },
        },
      },
      tenderdash: {
        docker: {
          image: 'dashpay/tenderdash:0.34.3',
        },
        p2p: {
          port: 26656,
          persistentPeers: [],
          seeds: [],
        },
        rpc: {
          port: 26657,
        },
        validatorKey: {

        },
        nodeKey: {

        },
        genesis: {

        },
      },
      skipAssetLockConfirmationValidation: false,
      passFakeAssetLockProofForTests: false,
    },
    dpns: {
      contract: {
        id: null,
        blockHeight: null,
      },
      ownerId: null,
    },
    dashpay: {
      contract: {
        id: null,
        blockHeight: null,
      },
    },
  },
  externalIp: null,
  network: NETWORKS.TESTNET,
  compose: {
    file: 'docker-compose.yml:docker-compose.platform.yml',
  },
  environment: 'production',
};

module.exports = {
  base: baseConfig,
  local: lodashMerge({}, baseConfig, {
    description: 'standalone node for local development',
    platform: {
      dapi: {
        nginx: {
          rateLimiter: {
            enable: false,
          },
        },
      },
      drive: {
        skipAssetLockConfirmationValidation: true,
        passFakeAssetLockProofForTests: true,
        abci: {
          log: {
            prettyFile: {
              path: '/tmp/local-drive-pretty.log',
            },
            jsonFile: {
              path: '/tmp/local-drive-json.log',
            },
          },
        },
      },
    },
    externalIp: '127.0.0.1',
    environment: 'development',
    network: NETWORKS.LOCAL,
  }),
  testnet: lodashMerge({}, baseConfig, {
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
          id: '36ez8VqoDbR8NkdXwFaf9Tp8ukBdQxN8eYs8JNMnUyKz',
          blockHeight: 30,
        },
        ownerId: 'G7sYtiobAP2eq79uYR9Pd6u2b6mapf4q6Pq2Q3BHiBK8',
      },
      dashpay: {
        contract: {
          id: 'matk8g1YRpzZskecRfpG5GCAgRmWCGJfjUemrsLkFDg',
          blockHeight: 42,
        },
      },
      drive: {
        abci: {
          log: {
            prettyFile: {
              path: '/tmp/testnet-drive-pretty.log',
            },
            jsonFile: {
              path: '/tmp/testnet-drive-json.log',
            },
          },
        },
        tenderdash: {
          p2p: {
            seeds: [
              {
                id: '4bc75fcb13e37ad6473383ea92408a451ed1b6d1',
                host: '54.189.200.56',
                port: 26656,
              },
              {
                id: '173fcd535bccde1ed20ca8fb1519858dd5cef618',
                host: '52.43.162.96',
                port: 26656,
              },
            ],
          },
          genesis: {
            genesis_time: '2020-12-30T14:08:02.904199237Z',
            chain_id: 'dash-testnet',
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
                  'ed25519',
                ],
              },
            },
            initial_core_chain_locked_height: 415765,
            validators: [
              {
                address: '1EBEA21DD88D3BE63AF73FBF3C63E0924C383993',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'i5jzHe+F6Y2KXB+y6aqjMivzRbsWnViFa0isQtjcMetQ+I+/4DeV2bgXrbhEjKO3',
                },
                power: '1',
                name: 'masternode-1',
              },
              {
                address: 'CDA301F28F1EFE8D4777B66F32359C0758B23716',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'CHlEEQPlC/Tgt50RytPMSXUzKpgV7b99JCfbo5SwIjarnkeiTjo6p3vQWL55pe4L',
                },
                power: '1',
                name: 'masternode-2',
              },
              {
                address: '98148E4FAA4E0FEF352A6395C06726BF3D8DCF12',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'DujBMSzFxG4YY9fy8edR3AkPyv48z+aEY2X/yc7H+2ZxLX2cQqiH7rr27kQpAZIr',
                },
                power: '1',
                name: 'masternode-3',
              },
              {
                address: 'D39A8DE0F43D5593C78084712BE51B85E3F5ECA5',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'jDGeJvJ/anuztdlhNnh6Lh5dBSSS6enep/N2OFscqwGRUqG2LfGeQt+xwDD++4Yg',
                },
                power: '1',
                name: 'masternode-4',
              },
              {
                address: '030A38B3D3BEDD513B5B688688EB3FCBA45891CF',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'CvSOgEMJhw3kYqVMSClcEjazTz5YKDQF2dSZQuQcoOHYjVy06T/cz4RwJFwKArhF',
                },
                power: '1',
                name: 'masternode-5',
              },
              {
                address: 'EAE52660FCD29ADBB9413F8C62B2AEB1530BA930',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'CUuqwqy3X6DhqStIjOGm4pzMmICKkGN+clskdedJMClnLpXMJw1WUTVHqah1htgA',
                },
                power: '1',
                name: 'masternode-6',
              },
              {
                address: 'C42BB57CCCE3FBC1B961A25163983DAA49BA7E9E',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'mLN3dHV1iOMZ+6HH6flCGziUCsAQimxA4SBBcIwe2bf7a4z8KfkdZMuwSoB6ku/T',
                },
                power: '1',
                name: 'masternode-7',
              },
              {
                address: '6475387F2739F3427CF303E0509D7D59FF42C83A',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'BmRwmqWjzCwn9h+rdMjstD56OFbsG+vf7Qw5kH2GQqqk5U2Ap4RCw4bOA8PKVP4/',
                },
                power: '1',
                name: 'masternode-8',
              },
              {
                address: 'AABF94B323D690751BE0FAA6E2428D0DCFC12FE3',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'lA9YNOjuuHxQqQrX6ju/7PkvQzRihkurZL2m898nwVus9H0+nMDtfdwIhqX0bjpH',
                },
                power: '1',
                name: 'masternode-9',
              },
              {
                address: 'CED6DD8FD612FEBD5C613B5C982544A57E452793',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'Fyff3hc0+9/xNCtwfx0jZ/SZLsYeeDRqkf8wQ2vHGeq97+2bht5GjiZenWG9GJcI',
                },
                power: '1',
                name: 'masternode-10',
              },
              {
                address: 'E5F9C7E6CC8F6998ADE638F3AD231B3BA84F2CF8',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'A7di/Va23m+WBe9EkcgdVPukH2IOqj+eyfW58bf2Oh3ieI1pDGb8vsIRVKbxp/2d',
                },
                power: '1',
                name: 'masternode-11',
              },
              {
                address: '8172056086CAFC834B62ECE3CBA7422993ECBEC3',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'igJ3TVw9IrLCcoWeaGccz74XZs2k8IogSHmn6dyT2Ds6EqkUznCrX3Am7qPXWaC2',
                },
                power: '1',
                name: 'masternode-12',
              },
              {
                address: '4ADD35B17299141F453853D291892F0AB5749430',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'gGlwT/PBXvdhkt/aw8Pchmtc+dRcxXTpFyIBA194cK76VcFhoL+4B3fErtqro1q0',
                },
                power: '1',
                name: 'masternode-13',
              },
              {
                address: '5A639948DB72533FAC6099661ED14440E589CDD5',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'GdmemSYPhtxOhDdz/oiNWTRGoBXjKIMEznQb2UQi1hmoe4+pWzy7+f8IB5Hir9Rx',
                },
                power: '1',
                name: 'masternode-14',
              },
              {
                address: 'BA447D4EA32286A8DA316F6BCC206092D09BD4A8',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'gfNfKbv1H47o0Bbb6Ot5NFOLTwsLDN0M54Q2cNiA1fvUBE+Wg7upIvJI4KyICXQJ',
                },
                power: '1',
                name: 'masternode-15',
              },
              {
                address: 'E5DE61BA6AC60B21D7B3D4EA7B5F8070CEB96960',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'Fb1kgu4x8JALUdoEPsOFYQDbyIuKkGTBNiWTQiDdtu+ORfW7k3xX6v/zjL+11zVh',
                },
                power: '1',
                name: 'masternode-16',
              },
              {
                address: 'B4DFB2B1031E05C340FCE4CAEA89ABA35DB3E343',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'CFXmkqPlcevJiXq6Zs3rllKgvW1yiLUe/tcEq243m2h2mU3tmAoWXgzWolXKnaPf',
                },
                power: '1',
                name: 'masternode-17',
              },
              {
                address: '4DBB37DFA38A1C1D112C69A2C9CA2CD266DCF257',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'CUD+5l8sjyrL+QD7qs1hwk/FpirXDMIQuY2pFlmJtrJKUi5PMBMm2JN5lyjyjjRH',
                },
                power: '1',
                name: 'masternode-18',
              },
              {
                address: '989D1396E454B41B78057C8BFEFE2E89E208E92C',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'AEllXrjmwj9rmokacAYqCspuWRl4FCk/SzV1C1s8B4c+u+zT0anm039DbL5hX3Ur',
                },
                power: '1',
                name: 'masternode-19',
              },
              {
                address: 'F7CBA67FC84F30CB41D43541D1533C37646A1AA7',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'gZLbBsgUicmMa/v3OMlXhP+NBQ13wX+Itsmf+bCcJhLfZ/L8gkkTTDvcW6C+lYOE',
                },
                power: '1',
                name: 'masternode-20',
              },
              {
                address: '5967C19F4F9FCE1C6D9BF4DEF8515BE8CC077F0C',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'By6RyRnzqOkwH67eSKZHEyBDUu1nTzcKTeWG+d4Gscq5A49vvi3PpMGIAclL9SoB',
                },
                power: '1',
                name: 'masternode-21',
              },
              {
                address: 'C1E943EF10312454277CF0A7837B9F773D124D9A',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'DB7Er6FvLg8uwT5Gh9NoCl0aAZD64p0ie/Y5xQXMuUvGE1+uR/nrBTlbU9IbQYPO',
                },
                power: '1',
                name: 'masternode-22',
              },
              {
                address: '6E8F97C1DF921F1FC510003F26E7CE6E2D083EEA',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'hefH4frYjhDCISbG99ZHqL8BXu/qK4L8iauAY9CqRHCtbclSeDTk8dUfGGFEIsC7',
                },
                power: '1',
                name: 'masternode-23',
              },
              {
                address: '8B0052703AA5EE4CC8A603CE7312B9889F04C831',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'F3vbFAZ0W/08mclOj1j0heBuA3gyr+VFR9RMg6iqZe2HY5Lym1oDPrqH12QHI5Iy',
                },
                power: '1',
                name: 'masternode-24',
              },
              {
                address: 'A91FF05D096BEF19A7BFEEC8F5249B6A44A7E0D8',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'EIIJW3MBY7Kp6YVVbwVxEejr89VsFFLRLbUQZ2NHQdB8lxtXPX/E1dzfYCcXHWXh',
                },
                power: '1',
                name: 'masternode-25',
              },
              {
                address: 'F93F6EF89B0C92D4B8EEBCD8C25E098CB321102C',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'EwBTb9osBFEjHHAC3nbYvkO5ogd00C4y6j8jO3z7G6fs41PRH4myYLqgjR9Tu/TO',
                },
                power: '1',
                name: 'masternode-26',
              },
              {
                address: 'BE5B9F1734CFB696C230FB3039A3EE9B828DD7DC',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'hCHdKIZmhaH8l4ZPFRYRdSRI4ZY+/iU2BPrigKMiC/snZXzaPJ/d+venQUYjOnVa',
                },
                power: '1',
                name: 'masternode-27',
              },
              {
                address: '68666401A05018E50D8948B1066A8C3F9F5D197C',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'kqs/IZSxhGm59Vtesz3PY1oyZpr0nDmWTXcGm5iC5l6OEd6WvHHlym3+Y7nKzDxL',
                },
                power: '1',
                name: 'masternode-28',
              },
              {
                address: 'D017EFA62E88FA2DEA801D01C2CB4FAC68530986',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'Fy0nV9ABqGtSbqBjGDVuSQ3IhXELiPf3vL8Xe1zXuyk+qWUapfteD//FncqfqWVa',
                },
                power: '1',
                name: 'masternode-29',
              },
              {
                address: '1C0E0B3D4E916A39C8FD3A0A93009C7E9DB51E45',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'CJF96qHDpIZ9hMEcZYTj3tLixpBBgwD+BH7NoFWBITMDEkrC3pAYgFXl9GLuyLd4',
                },
                power: '1',
                name: 'masternode-30',
              },
              {
                address: '49F1751ED85E96EA3139AA6E47A1A87155C89F3B',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'GfH3f+vDuRjaspLldbxe0r9z66Wr6AGlShZ5zABvWkYDt1skN4MhuP3+KC54Ybr5',
                },
                power: '1',
                name: 'masternode-31',
              },
              {
                address: 'A2A74CBF859881EEE8ED82CF2C447A4F02545E86',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'FcWrTNKGbMEa7FNQCGNUpDV+RyPJtY/tvlHrFVYZmygjJtLBL24H/CRI+t5ppNtZ',
                },
                power: '1',
                name: 'masternode-32',
              },
              {
                address: '3584086E58E9439A254DE1D023F30579E0ABECDC',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'Btx2e5/kpnc6ULExTdru3iX+DOsXGHPdh6QaxscpkbLs6FmauP+5lHnUXcozsQY/',
                },
                power: '1',
                name: 'masternode-33',
              },
              {
                address: '4145C30B756D6BC4CD44185AF164A381413EBEEF',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'BLd136AB0FiZkTEeWw1c5r4J7tqxk/iZqBbBYeTbmBhjIBbQtFVz8hV0EWtNiLuT',
                },
                power: '1',
                name: 'masternode-34',
              },
              {
                address: 'C4858F331785C217C7EB26C85ACDCC761071390B',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'gzYZml6XxJFANEYNUJq/dp8Y2WvWQJreLiP/GNY5lnixpqvFJnPt/ENX3fW5Ig5L',
                },
                power: '1',
                name: 'masternode-35',
              },
              {
                address: '92FE4F0727BB286983B32F67B9DA75B3727B8078',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'B5z3dYNwjhYPjVO7/RyCboJkA2FCPtWSpURSCs4DQeygRK5JxfSvTnVzwzQ+JbG5',
                },
                power: '1',
                name: 'masternode-36',
              },
              {
                address: '4CEF639E12437BA4811B07BFCCDF14A3A447B7C6',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'AMPJh/mc5FJEp/YV0sIzCUrJCdCgw8SQj3UgdIT4uqhS2GO1USyB3fqYZQD8MR0N',
                },
                power: '1',
                name: 'masternode-37',
              },
              {
                address: '0768F189FE5B7455A9799E14824399946DB0D9C4',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'CYGlY4yJNc8R/AY+6Ih0l+lInYCSX+fSBYuFvBa0ckOx4Ul/99bX8MOaR4Eh7Fgt',
                },
                power: '1',
                name: 'masternode-38',
              },
              {
                address: 'DC868031215A9FCDAD4DE50D1BC3F1CF92953E42',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'BKIYZzGvAI8XwD6j1lhqbPcG6UFcrm7wGIG66UA3SZ1gUsc1kf7p/2ZMZBOWUivD',
                },
                power: '1',
                name: 'masternode-39',
              },
              {
                address: 'B3F1DEC7BC6EBC6780C5A27A8841763551B58305',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'hDHzXRa5T/Ci+0q+ETy8G199Rj72ZvEZxlIyOwBrP7CAoc5MWwVo3DOutkdLEb9a',
                },
                power: '1',
                name: 'masternode-40',
              },
              {
                address: '04437C50C6B9FAE1A1906B7CFAB5192716388B67',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'kkjZpIrA99HJVWhFpKv89oUcDeCLtsAGOy0MY+V0tBVkLmltvwGKYYLJW3Lbou2q',
                },
                power: '1',
                name: 'masternode-41',
              },
              {
                address: '46C1599A1C8610D07BB4B7215720B04C4613B84C',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'kSIl1Zh077twJ6a+tviXsFzIsYXtQE/qZcCujppVeLz2NzSyvsbU5lF/WCp5vzax',
                },
                power: '1',
                name: 'masternode-42',
              },
              {
                address: '7943866D9F50D6229249D86ED31D344AC2DAB7F9',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'l2PeSRJAXTt/mCGlvLs7aFmLQIAKk6Rl0dbnjvCy86nLtUJSFsIQJ2yd0hYrTFGY',
                },
                power: '1',
                name: 'masternode-43',
              },
              {
                address: 'E3ACCB76F4455C8B384AF4D63244EB78A1FF6060',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'jZoe0v4sjAOJql7MJW+3FHe14lL8HsyBNeQELADa8edHTjBsQ5T6Bs/hXB9DK2Uf',
                },
                power: '1',
                name: 'masternode-44',
              },
              {
                address: 'B77DB22D3D089467B1496D55C3872FA02665502B',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'EEUPwsStCtNIICeUYzSzi6K1qrrRWYmkstI6zqFYZv1ayCMc08SSDzXBYfdLPWoJ',
                },
                power: '1',
                name: 'masternode-45',
              },
              {
                address: '1BC727ED0A48DE8B87D2FCF52BCBD7ABF06456C5',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'lxYemf6KwM5QkEpmp+ocxjQ6sH/YQ5pS7CMR5W7GXWPvH+In20gS0jRS1EGSd8PC',
                },
                power: '1',
                name: 'masternode-46',
              },
              {
                address: '44C96778285042612D445253DBFD8C7D01C508BF',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'FEO/iJ7UoJAF7FlqA4lqLXdx7m5T1DkxH8SPew79ZahQjbf5JWIDC+9g1ypJ75xO',
                },
                power: '1',
                name: 'masternode-47',
              },
              {
                address: '84B2CC28A97772BAC263BE89B1C3E7EDF3685560',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'ERb5RWY84nf115UUNqF5EKyluRaGm/iObWl9zzF87+Q/oQhpJlMOrN/pWk8dAK6B',
                },
                power: '1',
                name: 'masternode-48',
              },
              {
                address: '31E6E408485C3B0F28069BDB143D903BE59B710D',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'mWgQhnc6Gd+stIc+k0QCuFpp9ZOyCNdUWorC6OBqCTOUF6WQ6S88KscUa5MfyXlo',
                },
                power: '1',
                name: 'masternode-49',
              },
              {
                address: '0E955EAF6D7ACCB09DB3E5F865551E4E63B7F03C',
                pub_key: {
                  type: 'tendermint/PubKeyBLS12381',
                  value:
                    'Enw3AkvMVDKa0frk2Byp0Ok6E8ztRWl5aOOiMO0aeaWVZGNui51wE9jv1xRLOpMs',
                },
                power: '1',
                name: 'masternode-50',
              },
            ],
            app_hash: '',
          },
        },
      },
    },
    network: NETWORKS.TESTNET,
  }),
};
