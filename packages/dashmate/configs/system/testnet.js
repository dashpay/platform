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
        id: '76wgB8KBxLGhtEzn4Hp5zgheyzzpHYvfcWGLs69B2ahq',
        blockHeight: 59,
      },
      ownerId: '4yaJaaeUU9xG6sonkCHZkcZkhcXGqwf5TcNLw5Nh5LJ4',
    },
    dashpay: {
      contract: {
        id: '6wfobip5Mfn6NNGK9JTQ5eHtZozpkNx4aZUsnCxkfgj5',
        blockHeight: 71,
      },
    },
    featureFlags: {
      contract: {
        id: '4CTBQw6eJK9Kg7k4F4v6U1RPMtkfCoPbQzUJDCi85pQb',
        blockHeight: 77,
      },
      ownerId: 'GkodzWzGU9v4aCfc4qmzw4PeNCH8pqcxcEQrm7B24rNw',
    },
    drive: {
      abci: {
        log: {
          prettyFile: {
            path: path.join(os.tmpdir(), `/testnet-drive-pretty.log`),
          },
          jsonFile: {
            path: path.join(os.tmpdir(), `/testnet-drive-json.log`),
          },
        },
      },
      tenderdash: {
        p2p: {
          seeds: [
            {
              id: 'aa4c3870e6cebd575c80371b4ae0e902a2885e14',
              host: '54.189.200.56',
              port: 26656,
            },
            {
              id: '81c79324942867f694ee49108f05e744c343f5a1',
              host: '52.43.162.96',
              port: 26656,
            },
          ],
        },
        genesis: {
          genesis_time: '2021-05-06T16:28:56.385117522Z',
          chain_id: 'dash-dash-testnet-2',
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
              address: '3FB8B07867CCC4E30628DBD0A94FF5DFDB93DF07',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'EDJsyuN1r5cdIEl5JrVtDG/WNR2t9eL4d5BdtmLLpv7fLGfqLOYmawjHaq5KkpVs',
              },
              power: '1',
              name: 'masternode-1',
            },
            {
              address: 'E212A0CE251B9C112121ADA2C55170E49270B62B',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'gPeCHnIHfln1f0zhDGtCR4YT9nCVfYY+jP/ekF9a5iR4UqwgA2Fni6mEWDXxz/oT',
              },
              power: '1',
              name: 'masternode-2',
            },
            {
              address: '8772470A89B36858874D6414A580DD968A3BA0BB',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'GRy1esJVXIX1ecnlrN3LEQYuon7Rdwyo9DnbMiHe0t+An5zpNBfa3woUC4uavlhP',
              },
              power: '1',
              name: 'masternode-3',
            },
            {
              address: 'C572E4A96E38663151E2984AD08779F9E525D2DB',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'gpg7S9w2FR4tLiZmG0l7mrVqRUuidoFTjyR9GsVH3XVZeejcxKQVC9MsR9tKpRWo',
              },
              power: '1',
              name: 'masternode-4',
            },
            {
              address: '653B6C3910A716FA7959200F848BBE16B98C292D',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'BQr7IrX/PMdd1mKlky/LDtjHJuAACPDZRbF9PZUJnK4lsHLatSDLWrO0gtdc3Csh',
              },
              power: '1',
              name: 'masternode-5',
            },
            {
              address: 'FFEBF8C8516123AAFDBF38A1805D3E309F58D0EB',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'BNTJrk1X4PIuW3cmdtcMUx/12NOMSa7h1mLT1GQu8mnq9cG1vTR8QnJGF7TKbDe6',
              },
              power: '1',
              name: 'masternode-6',
            },
            {
              address: '0C25155268048921636E160AEFBFBC07A7367149',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'jEhInUfX+SxSgbO93eeyQA7xMwKfDqfjeqdUTe9WBeWIQgPmaB8rCqOuDUz62QFc',
              },
              power: '1',
              name: 'masternode-7',
            },
            {
              address: '780474EDC42AB9C385753E01AE2E407B65B34DF0',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'lorarv7ec1pIvHFrYKkCKVXXIpVCRuCvL8c8rHEHKvF7VD6nuMk4f+C1TxQw2M7t',
              },
              power: '1',
              name: 'masternode-8',
            },
            {
              address: '130D7CAB09C399543BFA23F906425173B7C956EC',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'Dc7R8JpiiC4gaVFnigBbKFIqtbj6rYSPvQb/NyeDMw/TXglTu6oQNB2chYBiyt1n',
              },
              power: '1',
              name: 'masternode-9',
            },
            {
              address: '7C8D95F4ECF7DF91FE456D1E8EC3B6EBF50A044F',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'jprbGM5zEe62FoCsC3xPsDw/WPimRvyW+vHrg3hJmbB1B95JVSDXhlJG+nh5PgOe',
              },
              power: '1',
              name: 'masternode-10',
            },
            {
              address: 'FA65A826CADC7121BC5975CA14729D449971A02E',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'Egp2UZNteF60cgd8qiRK2ZCG2Ng+vNvvwAQnG5+zuCJeHqZleVlCondEDyaIPLww',
              },
              power: '1',
              name: 'masternode-11',
            },
            {
              address: 'ADC542CB9E585E0A9932B76D37F6E22503E936F4',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'CnCCaovxDHldeHrZrmzkC2+HNAau+TyJla5chZ/9ePDY4q5HT1PrAtr1am6+c+la',
              },
              power: '1',
              name: 'masternode-12',
            },
            {
              address: '4ECAD50762EF361E77BA9242FA8AE536C9369E3E',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'EgLb056Mj+HyFxDw/2r7fCI/qjeUIcXb+N3RP3kcyw2apsl7q6QrwJi7DDnQUCSp',
              },
              power: '1',
              name: 'masternode-13',
            },
            {
              address: '5435BA428DC0C71B25B289F8B98C299B88DB8AEC',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'gqgyKUAuSPfZJJ33mzW/TVeon+Ltn9HuViGKhWvJwtHU3hpgpWqL0bBtN+mNUz7m',
              },
              power: '1',
              name: 'masternode-14',
            },
            {
              address: 'B792FF3A7A05AD513A5C76E57ACD007214E0F978',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'hZ4CAlBwHL/TbmRCaiJQHcQEk7HpmRzsNelaS14m5oxv6llhZALWxgL42PEqSCBZ',
              },
              power: '1',
              name: 'masternode-15',
            },
            {
              address: 'DB689C55F81FB3B877A2E29F92ECF0DF624BA0F8',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'kliOiW/zWqQoLz7VeqGiXRqDifaq2HmfLakqb4J/vGiO+bE9GarVdpxLjlOBDZZu',
              },
              power: '1',
              name: 'masternode-16',
            },
            {
              address: 'AC808E82CC1920154C7C56F2728229F6186B7EA3',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'lYTEiH3Fr05z3oRt8hTOq05ruJ4DCdymG1nTJxsMLgM8W3a2QHgXRgKHVLoHyiRf',
              },
              power: '1',
              name: 'masternode-17',
            },
            {
              address: 'CD1049EA61AD092865A5D276E069F6151A125EA4',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'CMd94Mx6QZCNkpHAMwOrNHE6+6P61Zaq/sR2btkim6RrdpKlsx7zZ35X8WHvv4Hr',
              },
              power: '1',
              name: 'masternode-18',
            },
            {
              address: '8C3AD3B4FDF2C40636BF34E6DD26D1711B7B8A63',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'hqGtHHF7xL/BD33Lo027zmdRebh1wJxSGjHNDWbh+QemBu5VTV4FPaJ6QdVqg77I',
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
              address: '69A53C9C0450105E3D4B611A1B3D561675C279A6',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'gPgSnhPzeErVPju2KGTXJzJzx8E6Xn9syMGCVUsTSRtCIqsG3TtyVCcmfiU5iZWn',
              },
              power: '1',
              name: 'masternode-21',
            },
            {
              address: '4DB233922817232ECDCDFB256EA0F8B6A0497DA8',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'gDiUGis0sowBvQCDvCeQDFu6rylNeYsE6ORaHaZYwTv1PWrjeOmZkGlk2m9Tc4xN',
              },
              power: '1',
              name: 'masternode-22',
            },
            {
              address: 'EC47E2D9E64A818BE0B6E22738FB1231879BAB51',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'lAQ+lLTEamPbna9xaXpBJC4dfHGOimT0MZG0Ee/CwH3GX/wyIDtXRK2kGkitFTa1',
              },
              power: '1',
              name: 'masternode-23',
            },
            {
              address: '85A839F4150FD010954A124B5F7DC7287FAE2ABC',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'mZDMo+8rGVz2jQ1AyJNeyghiWZ6B/cx4MbpOAa5m+TQRlVKVq2mI/gakzrwf02xH',
              },
              power: '1',
              name: 'masternode-24',
            },
            {
              address: '0CFB03735B9F3A661353861A9778D4437E014D78',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'lNdDoEU3G0Xa8zgDa4MBgkCghI7GattY4EjoS4/B1yJtjYKDFM21dTQTR2PZKghF',
              },
              power: '1',
              name: 'masternode-25',
            },
            {
              address: 'C1C853BE786E8EB70A277B94A279CF34AA2F898C',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'BOs3G6Zlcloa+3mkxwpLTuXxuWhBs058DtE8K9COAJu+KazeZGyGaSW4VHtM/6E8',
              },
              power: '1',
              name: 'masternode-26',
            },
            {
              address: 'BE4346DB2DA3EFA1C72EDF2BBEAC536B624FE01C',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'BE6dZrQNZLr8SQiSot6kpHaapWKODI/iOi2VfCrGrzM/6IiIuy0jvm4XjHfPfnqg',
              },
              power: '1',
              name: 'masternode-27',
            },
            {
              address: '1C6E273649E8CCB32E4C1BF715CC0334AD76CF8C',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'DVP2TfiY9+2p3r/9y2+BB6iD1pBxpAavGWXLZ8JZZyjBbDPl31QjdmYL5zjebMbo',
              },
              power: '1',
              name: 'masternode-28',
            },
            {
              address: '639D7F33AD2993D6883C3605EEA66BAE4F31F5F2',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'kjCDXyBtkiqFsLDZxzrOd6lqc9/W18vSqTuKYHt7jET47ovgpnu8FY5sGQtYF3aw',
              },
              power: '1',
              name: 'masternode-29',
            },
            {
              address: '87B0405FB9E1C0BE84DFFC8D678A96CE95265EF7',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'BFPTwXu5icm/veDYdzzPc+56oAwYNH53xMknQpfrnSkF7gFMfCbsY+Y61kfOh6BG',
              },
              power: '1',
              name: 'masternode-30',
            },
            {
              address: 'E561951B266689615499C767B34793ED8F7091F3',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'gE4acBzBnEi+Tns1TzJEvmXah5OSpoFlWMaRcfWU+vEpa6JdcEC1hZtC4IY+LQqw',
              },
              power: '1',
              name: 'masternode-31',
            },
            {
              address: '3A02F4D0BBE65749AB937307F2191FB5E7200BCE',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'hGSWb3/JPpICvDLuckSmUIcDbqxHTMfJNy5f2jiiFt0cpVNko4sjM5dZfSB4QvXX',
              },
              power: '1',
              name: 'masternode-32',
            },
            {
              address: '12AAA6752191F2226342C6C1C251A2B3078946CA',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'EwpuqAQJIhPJhD84PzsGaGEafldXetRQ8cIcDkE92RJadWrM3gDC5cbkSX1Ame5n',
              },
              power: '1',
              name: 'masternode-33',
            },
            {
              address: 'FC034769ED1930CA5C0D7F28E24348FCEFB3DD6B',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'D98xj5j8L4p6xW57+BUMBT929VBa8jmqH5V7n5/SbMJCyCh4adJtVGwXZomq+R8l',
              },
              power: '1',
              name: 'masternode-34',
            },
            {
              address: '2B3366F08C10801A78F8C8132695DB0CA700BF26',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'lwAE66OfX/Z3MUBJ9/MshjagP5tq/1rBm5Zx9A3iizqKO7UMmnjt48TJemq/shC5',
              },
              power: '1',
              name: 'masternode-35',
            },
            {
              address: '003A49B44CE04A18D2B973E1761C868D59BF6965',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'CHxFeYUREcuHCEwuyYgs/DhvJmQsPlzsHZOIIe/YSbvOUTJHOpQ7LqQYTQ17eDgP',
              },
              power: '1',
              name: 'masternode-36',
            },
            {
              address: '431FF615AA37F85C79CBC6F52F0C7BF16E54CE71',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'hNZjWJiMzKBQWid/foYDRok4DieVcKmEMyUVJbUuS4zwY3IWUV133mr0fhUxnxbX',
              },
              power: '1',
              name: 'masternode-37',
            },
            {
              address: 'C2978A4DE5B09509BDEC36A809D4F406A280F50E',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'hcVXMlqXvY+EaSziGaYSvxA0MpAwxgGyazXdRIoBlDNC/aoAaYRYh1YgWTzJ00AP',
              },
              power: '1',
              name: 'masternode-38',
            },
            {
              address: '8424D3CFD9666011D02C52B0DD7DE43D0C2B5FCA',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'ES6bs4uy/nB0UUMbsjLf1W0zn07FXBBxZhSDoEERWN3SmjQ8hDg8DvYbeKplr083',
              },
              power: '1',
              name: 'masternode-39',
            },
            {
              address: '2A57D56D66B16218E8032543F203187FA452BAC7',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'hsKQYDAzx02HaH6/C7r+7hFrkMEnuI3w98dmKuJUwLG7ejkHlgffvycvvRJ7Ynz9',
              },
              power: '1',
              name: 'masternode-40',
            },
            {
              address: 'F43B832A018B6CC7DB371A277E56F7E1A92471BC',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'igXlIELWdQh4WmNkWEL8pgeWR2gf1kFommFYaQc3sfxNNu4lcRhlDjh1rS/RDqdi',
              },
              power: '1',
              name: 'masternode-41',
            },
            {
              address: '3F24F49B221D0CCA0875BB1AB54EED6B90575EBC',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'i02syBXFDGvrjLZuWLV1hfsB3D18WstauPReCSAe9g/uczDOAhPme0HvanQVUC4O',
              },
              power: '1',
              name: 'masternode-42',
            },
            {
              address: '0BCF1AF7C0AC734ED55E333133FF6160053CA401',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'ijjRF5BR72seoWsQjp1+VXVjFl5tKERRs8jbiOiQg9xcAahtJSxezinmcEnDcknm',
              },
              power: '1',
              name: 'masternode-43',
            },
            {
              address: 'DF112B36143798E7CDF0B35C9F3642B586313F3E',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'C58dv65RVcbST5NHAO+/UC6AzfeSkZA0VMjxttWpW7uGz9cx7ufoctJMPXSn/wo9',
              },
              power: '1',
              name: 'masternode-44',
            },
            {
              address: 'BF4A8CFA5B8B749A0815AE83396AFA8479148C38',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'jj3FRNpqVRZCJehv99li3AejT1D/j0/6u0rpk13m7C22hjQMjFaRusienjf84UGK',
              },
              power: '1',
              name: 'masternode-45',
            },
            {
              address: '60F10F23194263C3DAB9EDC1D502CD4B344739E6',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'EXw8Dw2OVPUGYq3HxNHdLVNFlrD9YwDjyk2RKT9ERcI+jizmjsweOcTxt3DL7RDu',
              },
              power: '1',
              name: 'masternode-46',
            },
            {
              address: 'A05507F5A17AAFE8371867CEC4B17F613B0D662C',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'DjVzih3sNq/wsF4QmD3kWSLnRZnD3qYpZMQI/qqQzX1SPOjiKAKpQaReu89DzD+p',
              },
              power: '1',
              name: 'masternode-47',
            },
            {
              address: '648440F7207C5F188757AF7C732C9A2912241A03',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'FD2mFBrBzmrMtPwS0dzhva7kCqhb4AgW/CsXPmGGJHvajYRwVZyHyLGKVsFNE/B+',
              },
              power: '1',
              name: 'masternode-48',
            },
            {
              address: '7742BFDD7630DCF89506DA213750A5465114CBC2',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'AH4jhMBX6HYIfCmIbVNE+hjw6qS49QjhdiidR5o0RoDyFlAw4FvA8P+6PZ9lUJ5K',
              },
              power: '1',
              name: 'masternode-49',
            },
            {
              address: '12BA0A782BB3E34029E2187B33F91F95DEE8A36F',
              pub_key: {
                type: 'tendermint/PubKeyBLS12381',
                value:
                  'ALFicBvONnfA3IrtPFKWpBpje3XYB7FSPnBhUZHBkVwQr3Fgs6tQ3lpn6nFmTNGh',
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
  network: NETWORK_TESTNET,
});
