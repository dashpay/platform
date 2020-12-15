const lodashMerge = require('lodash.merge');

const NETWORKS = require('../../networks');

const baseConfig = {
  description: 'base config for use as template',
  core: {
    docker: {
      image: 'dashpay/dashd-develop:latest',
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
          image: 'dashpay/dapi:0.17-dev',
        },
      },
      insight: {
        docker: {
          image: 'dashpay/insight-api:3.1.0',
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
          image: 'dashpay/drive:0.17-dev',
        },
        log: {
          level: 'info',
        },
      },
      tendermint: {
        docker: {
          image: 'dashpay/tendermint:v0.32.12',
        },
        p2p: {
          port: 26656,
          persistentPeers: [],
        },
        rpc: {
          port: 26657,
        },
        genesis: {
        },
      },
    },
    dpns: {
      contract: {
        id: null,
        blockHeight: null,
      },
      ownerId: null,
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
    },
    externalIp: '127.0.0.1',
    environment: 'development',
    network: NETWORKS.LOCAL,
  }),
  evonet: lodashMerge({}, baseConfig, {
    description: 'node with Evonet configuration',
    core: {
      docker: {
        image: 'dashpay/dashd:0.16',
      },
      p2p: {
        seeds: [
          {
            host: 'seed-1.evonet.networks.dash.org',
            port: 20001,
          },
          {
            host: 'seed-2.evonet.networks.dash.org',
            port: 20001,
          },
          {
            host: 'seed-3.evonet.networks.dash.org',
            port: 20001,
          },
          {
            host: 'seed-4.evonet.networks.dash.org',
            port: 20001,
          },
          {
            host: 'seed-5.evonet.networks.dash.org',
            port: 20001,
          },
        ],
      },
      spork: {
        address: 'yQuAu9YAMt4yEiXBeDp3q5bKpo7jsC2eEj',
      },
      devnetName: 'evonet-8',
    },
    platform: {
      dpns: {
        contract: {
          id: '3VvS19qomuGSbEYWbTsRzeuRgawU3yK4fPMzLrbV62u8',
          blockHeight: 35,
        },
        ownerId: 'Gxiu28Lzfj66aPBCxD7AgTbbauLf68jFLNibWGU39Fuh',
      },
      drive: {
        tendermint: {
          p2p: {
            persistentPeers: [
              {
                id: '08dd8e2b1968c1323b9460949971132653ece7d8',
                host: '54.69.71.240',
                port: 26656,
              },
              {
                id: '622758cfb580133f2a33b1a9e7f3bf1591da299d',
                host: '34.212.169.216',
                port: 26656,
              },
              {
                id: 'f39664e7d911c42ce17cc0f4c8cc322f1b72c29e',
                host: '34.214.12.133',
                port: 26656,
              },
              {
                id: '27904c3833d4d8d436152bb9138fb7a18fb071fe',
                host: '54.190.136.191',
                port: 26656,
              },
              {
                id: '23e501d4eaf2edc1d5a49f6349327e9d4fb4ccdb',
                host: '34.221.185.231',
                port: 26656,
              },
              {
                id: '520378c6ba73aebab3d1e25388d1b9a6ddc2ad86',
                host: '18.236.73.143',
                port: 26656,
              },
              {
                id: '84b58e977d1e50fb345e03395d6a411adba74da9',
                host: '35.167.241.7',
                port: 26656,
              },
              {
                id: '85f7bb17e40a49d6724504ca5caa6df8e42b74e4',
                host: '52.33.251.111',
                port: 26656,
              },
              {
                id: '2570192e340943293eaba0d7d71182de01fe79a6',
                host: '34.221.226.198',
                port: 26656,
              },
              {
                id: '7ac52a4464c5047d22a84eca3d218202277e6db0',
                host: '54.202.214.68',
                port: 26656,
              },
              {
                id: 'b575d09fe651357dc677b5d5c2ec0982251e7dca',
                host: '54.186.22.30',
                port: 26656,
              },
              {
                id: '86008634cb79d53fb6c286a6e682c0891328ac2e',
                host: '54.186.129.244',
                port: 26656,
              },
              {
                id: '4ab0e5e0c56adf0347d17f67171be54cca0a02e9',
                host: '54.186.145.12',
                port: 26656,
              },
              {
                id: 'c73217945855cedf1fc196b584fa5632538e0664',
                host: '52.88.52.65',
                port: 26656,
              },
              {
                id: '9659193b7d35b0f4935e0ce81e0baed15c19053a',
                host: '54.190.1.129',
                port: 26656,
              },
              {
                id: '3527b5004648540a489492f1e9a5aa272bc0832b',
                host: '34.216.133.190',
                port: 26656,
              },
              {
                id: '9e9e0b8252bd58ea19caa31737b2d2172fb0cef9',
                host: '34.221.5.65',
                port: 26656,
              },
              {
                id: '2b1817b5eca46f5852a4d4865c5bd8f007a85848',
                host: '54.149.181.16',
                port: 26656,
              },
              {
                id: '1ac504d40d879437250622474b636ed1ca723945',
                host: '54.203.2.102',
                port: 26656,
              },
              {
                id: 'd8eb0fa388c3753c9a8444d421a79bf8e6c7143a',
                host: '18.236.235.220',
                port: 26656,
              },
              {
                id: '49fc482748d0426146e21a7e453cc4c9c2438d54',
                host: '18.236.139.199',
                port: 26656,
              },
              {
                id: 'd748a3334109e51793ae735220e19b03fc1ed568',
                host: '34.220.38.59',
                port: 26656,
              },
              {
                id: 'fe6d601370186e71fc1812d3f616b223137fc9a1',
                host: '54.244.159.60',
                port: 26656,
              },
              {
                id: '62a0c0d795424081d9cbb5ab1296398859b736eb',
                host: '18.237.255.133',
                port: 26656,
              },
              {
                id: 'cbf9dad753193e6620d678368240aa5657ab530e',
                host: '34.220.159.57',
                port: 26656,
              },
              {
                id: '8200d0ca52f3296729232f1b741b24d8d7137232',
                host: '18.237.194.30',
                port: 26656,
              },
              {
                id: 'e0c512cd361e49c1117bc1f881223cb558bc37ab',
                host: '52.32.251.203',
                port: 26656,
              },
              {
                id: 'd7ff609df651cf448ad5fba7f35e2de61057ff3a',
                host: '34.222.91.196',
                port: 26656,
              },
              {
                id: 'a3dcfd041b12c6629c41a670377c7a945da3a848',
                host: '34.219.43.9',
                port: 26656,
              },
              {
                id: 'c1499a9650a96a8034e606df5b90f76b36051021',
                host: '54.244.203.43',
                port: 26656,
              },
              {
                id: 'a6045508e21d4946b0b6a8f9924882a6bdf0fbc9',
                host: '54.245.217.116',
                port: 26656,
              },
              {
                id: 'b88de2d0f943f4e0748039c0d6d72c6511032469',
                host: '54.189.121.60',
                port: 26656,
              },
              {
                id: '77316bb767e52216b777f171a1382aa573ec6608',
                host: '52.88.38.138',
                port: 26656,
              },
              {
                id: 'c1b6760ce4267b4ea7e90d54b0e9d4ab95483051',
                host: '54.185.186.111',
                port: 26656,
              },
              {
                id: '7aec8ce958561379eb85ebadec1f0c93c83db6e8',
                host: '35.164.4.147',
                port: 26656,
              },
              {
                id: '881b7d135b29797fcb652c7c7807c9f2b243fe9a',
                host: '54.188.72.112',
                port: 26656,
              },
              {
                id: 'f52237cfc900b26589969bd93b0ca775ebe00f76',
                host: '52.88.13.87',
                port: 26656,
              },
              {
                id: '84804842f805684bb831b5799d3628a5e31e6fef',
                host: '54.149.99.26',
                port: 26656,
              },
              {
                id: 'fc85b3e8212c17f231a332395a3923a7202ba794',
                host: '54.212.206.131',
                port: 26656,
              },
              {
                id: '856d39ed1d493a5436188c14bc2e11d87863f393',
                host: '52.25.73.91',
                port: 26656,
              },
              {
                id: '4ab81caa5be52118e442496e31b51d5e7fb1e020',
                host: '54.187.128.127',
                port: 26656,
              },
              {
                id: 'a97df28a6961287acac36b7e8632e9988e614841',
                host: '34.223.226.20',
                port: 26656,
              },
              {
                id: '74866833db1577a9916a09592f5ab76b455db15d',
                host: '35.167.226.182',
                port: 26656,
              },
              {
                id: 'de7241a53bb157cb78429e79b3074f87801721d8',
                host: '54.189.73.226',
                port: 26656,
              },
              {
                id: '7220ea2cc1474dbb979171932b191643dc508519',
                host: '34.219.79.193',
                port: 26656,
              },
              {
                id: '49b3cd3c6fd8eccd3e81b468512b91a87e22a60a',
                host: '54.186.133.94',
                port: 26656,
              },
              {
                id: '0786707a967d5032ce552d3652de9d724acf820c',
                host: '54.190.26.250',
                port: 26656,
              },
              {
                id: 'bb943d27e94ddeb18cef0bb8772df4b09a727e7d',
                host: '54.184.71.154',
                port: 26656,
              },
              {
                id: 'd45a481e4845fcca3f14f333803190eff05f5b0d',
                host: '54.200.73.105',
                port: 26656,
              },
            ],
          },
          genesis: {
            genesis_time: '2020-10-29T14:54:55.243362093Z',
            chain_id: 'dash-devnet-evonet-8',
            consensus_params: {
              block: {
                max_bytes: '22020096',
                max_gas: '-1',
                time_iota_ms: '5000',
              },
              evidence: {
                max_age: '100000',
              },
              validator: {
                pub_key_types: [
                  'ed25519',
                ],
              },
            },
            validators: [
              {
                address: '8AA28E29CBE6F8C44228AFF79212A05211531D31',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'YY8qJE2Jyl90SN0NeV4G+btces5S2MLxkdHu1dzK9gE=',
                },
                power: '1',
                name: 'masternode-1',
              },
              {
                address: '9776D772EF437DBC550BFCFDF00BA85748F73F05',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'Q2jE22KAsTg+mzcODSuQR7cNFyaDfhn2Bf5+y0Ssm30=',
                },
                power: '1',
                name: 'masternode-2',
              },
              {
                address: 'C8F7D628EED93D33AA6C264BFFB3CD9476058FB1',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'hS+U08pOWjxsvqVS3vq0qD8yjQUHWH1IbEjZpCWsnqo=',
                },
                power: '1',
                name: 'masternode-3',
              },
              {
                address: '09794F8EC5F1998CEA9C3E849F42EA0BC333F524',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'T5J1zf2kANtmtMpsu1EXl9OznOivWlIO5q1jpJ6B5d8=',
                },
                power: '1',
                name: 'masternode-4',
              },
              {
                address: 'BA486FE73F3242ED721C736E480261223FDE287E',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'HsEjBaDtaubMxjlG4iSIOqhflyKIY99BKkM9tcM9yOo=',
                },
                power: '1',
                name: 'masternode-5',
              },
              {
                address: '701C3E9F901B87A49BFA521BF9D504BB8C2BB98A',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'QPC+O0rctfMrfXMDsj9DnDEi8KypCED/UxtAbjK3D3I=',
                },
                power: '1',
                name: 'masternode-6',
              },
              {
                address: 'BFFB2EE1328E733BC6937EB0FDC48DDD88664C87',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'VoVlGN/CzXEkj27SrOVsE4TPYaEOjukUAM+HKKOX9p8=',
                },
                power: '1',
                name: 'masternode-7',
              },
              {
                address: '536D4E4C98F11CF73CE452516F88641E5E98C225',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'S8zd/4Y+Aa4USQqv4zQqxWsUh83J9kzjuGDMZGvs0mg=',
                },
                power: '1',
                name: 'masternode-8',
              },
              {
                address: '426AF75057755FA77DAEB57E84F112EC61BC0C23',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'VgptZSnRGd3mGj9Xw8ThWknkXiq8xchHoYsz5+PdvEY=',
                },
                power: '1',
                name: 'masternode-9',
              },
              {
                address: '07F9D83F23933B9BC5252E7AB0D6F2C1737330CF',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'nOsFaCMtxYtGPu7MsTNs+H8j5NAOFnbH8BEYKXRJOiM=',
                },
                power: '1',
                name: 'masternode-10',
              },
              {
                address: '1676685AF7DE461175E9595A73D5BE2481714EE9',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'IPz4FOI5QDIBQnt6+yohwpoI5hHLXy9dCsJ3UFpLhnw=',
                },
                power: '1',
                name: 'masternode-11',
              },
              {
                address: '7A2585A7E16BAEC975126E01568B722183D2E051',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'f7oR6yK1hYm/i7U06MvGHTFnuMEuHHnlSnqlXQRA8rY=',
                },
                power: '1',
                name: 'masternode-12',
              },
              {
                address: '8F9D9F55AD7EFCD1ECE143A11FCDF9BCC6264682',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: '5co5iELzJnRkppYLWEk31XBlBch8wcdV8FtCYeBf+GI=',
                },
                power: '1',
                name: 'masternode-13',
              },
              {
                address: 'CBFD58CED7CFFC61D6EE3A279E983D211C0639AA',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'rnwGhO9BGtMgAqkIIT4pnkVfrOhcXxsmeoFczXwhisE=',
                },
                power: '1',
                name: 'masternode-14',
              },
              {
                address: '267A351A6263D2C2BF18B7E2B94F83BF2AF0731F',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'vR7nVdkz1b03FxS5t29380qGEy+Rawl+dJBZQst6PUY=',
                },
                power: '1',
                name: 'masternode-15',
              },
              {
                address: 'FE28EDCCB8918F8EEB687771EB1A3B3CCFFBED1E',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'TdkmKdE7h694uzmY2ADVnlLP/QQUohnIhsiGN62U2e8=',
                },
                power: '1',
                name: 'masternode-16',
              },
              {
                address: 'D9A58D252185AFF88D729E82995E98A1805CEC5C',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'qjZIRbD7yU4b+0IQx7ZuAU7weA0+Nt6G1ZeEfpCrfuo=',
                },
                power: '1',
                name: 'masternode-17',
              },
              {
                address: '470CB581A6CBC3ED01AAD9411ECD7C2D9FD5C54D',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'eEQhtx3twf/Jk+QdASZ6VhTTCPB9MrJzvVk72IPRWys=',
                },
                power: '1',
                name: 'masternode-18',
              },
              {
                address: '2188A90F81D3BA3F8CBABABBA03696FE84598D88',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'Lkoq1y+9AaGAN3PzHTPYyALiofYijHeYO998fpGwEl0=',
                },
                power: '1',
                name: 'masternode-19',
              },
              {
                address: 'FCB9FE6B879F7320FB2FB06BDD461BEEBB846B85',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'nDAxHaxWUBxtCLWq8vxiAe0VFolR6Yw4HEgM9zC2egE=',
                },
                power: '1',
                name: 'masternode-20',
              },
              {
                address: '2B67AF00F7738FED12780B02F70AF766B9B5C0F2',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'nsEDH4zcbUC8Dufa17ZxRwAqct7diNoNzgYHb2TQ2mg=',
                },
                power: '1',
                name: 'masternode-21',
              },
              {
                address: 'E6F029EFB7B248C84B320B380B30619EAA22A65E',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'wHBzA9ic/90SbhODRbT9cWzbhqXoXu3X/1NgYKt832A=',
                },
                power: '1',
                name: 'masternode-22',
              },
              {
                address: '1A761A571BD1786897BA4D969A03528039C54381',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'z/wzEYKiEwxP/nkgN11WL6erfTEq4wcBulFCi0bsQgg=',
                },
                power: '1',
                name: 'masternode-23',
              },
              {
                address: '816F57EBC23F03F069E648ECC2E5FD5CB12F6C4A',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'jsB/NVdo24Rsl73P5ymOhSAgK+V7aBodr/EJdarBspA=',
                },
                power: '1',
                name: 'masternode-24',
              },
              {
                address: 'C304B9B9E76DEBFDDB71B66C83E0E04441C5C113',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'KSXR93tqocMmmAfUJY9DagiciomWXnZ+SKgAt64vImg=',
                },
                power: '1',
                name: 'masternode-25',
              },
              {
                address: 'A6CF545577FCAE91D92A091A572B1DFF7FD3A574',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'DxMddyEkMix5lu1CFGSSd+XskrFhq+ZOzXzBRUq/Yyg=',
                },
                power: '1',
                name: 'masternode-26',
              },
              {
                address: '63D4A3B2F0EA3576109A7BA8440C47559732BD92',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'NAW+eCntw8ssgjJ6yY/hGuAlgCMY5gKXPHbya4rPNfs=',
                },
                power: '1',
                name: 'masternode-27',
              },
              {
                address: '04462517B4EEE922E3EAC62D8FC3B39157D4CBF1',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'YLyrpYp/I7EhxCA9yzGwmnXw9VI5t1fv2H7PDpDPFiI=',
                },
                power: '1',
                name: 'masternode-28',
              },
              {
                address: '77A193DC1A73F7DF324482268CE3F03740DACC33',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'mbXb1wqBREq9j9TkX9Hn+ZyP6eJNRQ2UiWiwqgRxN98=',
                },
                power: '1',
                name: 'masternode-29',
              },
              {
                address: 'E9BBE3577D67A63C0B3DA23A8F65C2E4C388E567',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'kECCYzOnEcA5FiqYq7/+VaO/lV8nQuUhSR+zDMoHjW8=',
                },
                power: '1',
                name: 'masternode-30',
              },
              {
                address: '6D9EA83F44A8A309D2979DA0716B6E06CBD8D926',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'wi1tZegVxE/cFa8pjzZVUKInxwqbELarJjlSisuJoSE=',
                },
                power: '1',
                name: 'masternode-31',
              },
              {
                address: '4816F44F85A1EE7EEA6FC6CCC19AEC93631909FC',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'C4O3e/0wAOa902JtcC3okmLq0VkO88DXkbruLVwAc7c=',
                },
                power: '1',
                name: 'masternode-32',
              },
              {
                address: '794389AC17E4C4F92ED18D5DB5458150B0918835',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'TOMdmb/WlzaOp6hMe7vAF9Xbrju//b7az0QLhKraawM=',
                },
                power: '1',
                name: 'masternode-33',
              },
              {
                address: '6C1A743FF3393529A707B55F5EB649EA4FAD5980',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: '0PRvdsjn1APsMDUw/HT/MqdE3Z9HR877Dx60fk997y4=',
                },
                power: '1',
                name: 'masternode-34',
              },
              {
                address: '8EB9F71D65DF48396AF2F6770FD42BAD36A98F69',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'bUZR4IVeHgtfIksl5MTUbGJr2tBGSRHL0G+/vVpgBgE=',
                },
                power: '1',
                name: 'masternode-35',
              },
              {
                address: '50FF638F377821861790A61DA0D6A03FAB10223D',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: '+kCo1Ztc+JA8xq7xuj/ggVfGRQWBseTuf95omfB2HmU=',
                },
                power: '1',
                name: 'masternode-36',
              },
              {
                address: 'B7DEE29CF61EA9455E85E5CC88D39688732799DD',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'tjjQbJ10uwCoZYSvNbZgcEf/Er8e95ymoCsAwhp5Ndk=',
                },
                power: '1',
                name: 'masternode-37',
              },
              {
                address: 'BF01E4C420DB3256C3BDF561D6DE806A6A0FAF72',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'Ti4MSaMJIDO5TTMKyOCVn08ZIfA9558nf/4SuXbqijU=',
                },
                power: '1',
                name: 'masternode-38',
              },
              {
                address: '34A1F4C14D6682E010243E154AB1205F2277084D',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'bexU65YMRfR8b1fEkUwIOUKHNl9V8cT0soi8DDG90As=',
                },
                power: '1',
                name: 'masternode-39',
              },
              {
                address: 'C3EB398E6499A726D083EABEAB9653302F94EF90',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'iPHtYcQTAaq08hq5vf67ufYLNf6i6SW5f5echYKvV3c=',
                },
                power: '1',
                name: 'masternode-40',
              },
              {
                address: 'AFA41D67E999F628EE67C7F00BFF492682180A6B',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'fUF/43vvbaAcaEpuoJmrY66LerYNpTmE8rmE1fraxnM=',
                },
                power: '1',
                name: 'masternode-41',
              },
              {
                address: '80F3F1CB7B2F7EA0151EA1433E62E76B17D5F1A3',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'JIc05l8UK1kGzMeDUu4mhDPCDkLTwkymiQPgIrNqeyU=',
                },
                power: '1',
                name: 'masternode-42',
              },
              {
                address: 'D99C5CE2F15F53B2C4DA37ECCA6A427C612CB86E',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'QkBN043FReHmIndf0zpM1deZTSgjc6pZgLfswxi4LI0=',
                },
                power: '1',
                name: 'masternode-43',
              },
              {
                address: '94DC7A83BCEBDC14DCC8D9AA796A9F3C30758A5A',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'MGzwcGFyj5OM6BQfaF6qERvRwFVLwrEtIjzrD4HJgXM=',
                },
                power: '1',
                name: 'masternode-44',
              },
              {
                address: 'BE664A59B0FE59B6809CE6F3D74AFB6FF9F4F595',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'VwdQNhEKVHYUZK/9YyeZokdbG6xWbUH8UAZJ+0MMPkA=',
                },
                power: '1',
                name: 'masternode-45',
              },
              {
                address: 'B924A4F48AF9B0902E046055A0B9217EA7D8AB31',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'QSVnY9ZBIjh0eQg9E9TQ3C6MqAZcGkH+iCMz2+F4Y4E=',
                },
                power: '1',
                name: 'masternode-46',
              },
              {
                address: '1C4AC45AE3846BCB28427FF80C82E6A1BC66B786',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'W6R5wifl0maN1f+jGOAFRRyUphFtp/6os1We3rfVIAI=',
                },
                power: '1',
                name: 'masternode-47',
              },
              {
                address: 'EC910949C536C9F4EF512BF36A722D00C04C7A59',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: '6OcOw3+BmuW33tmcO8TiG1HLRsA9iMZkDJB2wJQs5so=',
                },
                power: '1',
                name: 'masternode-48',
              },
              {
                address: '856B7A347EB77DB7D9BE1F4748D0B90985B54CF4',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'TbiLMDredczYV6fJdAfrK4dAh8Gdn8rt61nP+od3Nd8=',
                },
                power: '1',
                name: 'masternode-49',
              },
              {
                address: '1E984D356B54AACC996D25C179F2506C18CCC55D',
                pub_key: {
                  type: 'tendermint/PubKeyEd25519',
                  value: 'pRkdsp3BT9Av2EZDzERzaM/ySuce7YnY5YJc8vVodRI=',
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
    network: NETWORKS.EVONET,
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
    network: NETWORKS.TESTNET,
    compose: {
      file: 'docker-compose.yml',
    },
  }),
};
