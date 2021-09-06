const { expect } = require('chai');
const { WALLET_TYPES } = require('../CONSTANTS');
const classifyAddresses = require('./classifyAddresses');

describe('Utils - classifyAddresses', function suite() {
  it('should correctly classify address for HDWallet', function () {
    const walletType = WALLET_TYPES.HDWALLET;
    const accountIndex = 0;
    const accountStore = {
      accounts: {
        "m/44'/1'/0'": {
          label: null,
          path: "m/44'/1'/0'",
          network: 'testnet',
          blockHeight: 554643,
          blockHash: '0000007a84abfe1d2b4201f4844bb1e59f24daf965c928281589269f281abc01'
        }
      },
      network: 'testnet',
      mnemonic: null,
      type: null,
      identityIds: [],
      addresses: {
        external:{
          "m/44'/1'/0'/0/0": {
            path: "m/44'/1'/0'/0/0",
            index: 0,
            address: 'yd1ohc12LgCYp56CDuckTEHwoa6LbPghMd',
            transactions: ["1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6"],
            balanceSat: 642650000,
            unconfirmedBalanceSat: 0,
            utxos: {
              "1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6-1" : {}
            },
            fetchedLast: 0,
            used: true
          },
        "m/44'/1'/0'/0/1": {
          path: "m/44'/1'/0'/0/1",
          index: 1,
          address: 'yMX3ycrLVF2k6YxWQbMoYgs39aeTfY4wrB',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/2": {
          path: "m/44'/1'/0'/0/2",
          index: 2,
          address: 'ydJGUUmNxdmvyskoZXqtJRqWyqsaFPitGQ',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/3": {
          path: "m/44'/1'/0'/0/3",
          index: 3,
          address: 'yhugNjDRVJUL7PK9MqQP9M6M1HJm8zuWJL',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/4": {
          path: "m/44'/1'/0'/0/4",
          index: 4,
          address: 'ySgckdRYxpa7Uda8yUNRqjYeuusqLy3AY3',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/5": {
          path: "m/44'/1'/0'/0/5",
          index: 5,
          address: 'yS4DeSU3MTBisgL6p8PDRzSxTPN2PUt3vE',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/6": {
          path: "m/44'/1'/0'/0/6",
          index: 6,
          address: 'yRZef5UGotGgLMaLYTzhvfknogqMRBkUiX',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/7": {
          path: "m/44'/1'/0'/0/7",
          index: 7,
          address: 'yTUw2bGzi9rYs41XH1dxbRqiJoDtwuUcv2',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/8": {
          path: "m/44'/1'/0'/0/8",
          index: 8,
          address: 'yNQHAGhNP7UbhxnkZH4muP2oKkuayGEuwX',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/9": {
          path: "m/44'/1'/0'/0/9",
          index: 9,
          address: 'yPPDLBDjHctWpMxLiMTJXLngcYPke7YNaY',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/10": {
          path: "m/44'/1'/0'/0/10",
          index: 10,
          address: 'ya2vVJAJdZN2We7MYiSjGf9wkdWF6A1RLr',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/11": {
          path: "m/44'/1'/0'/0/11",
          index: 11,
          address: 'ya5k2YMjfyfxZoidq4UdQ65jYUXvtVomEv',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/12": {
          path: "m/44'/1'/0'/0/12",
          index: 12,
          address: 'yhXZja3Apyp9S32zEVsPqLssNJZLrczxJC',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/13": {
          path: "m/44'/1'/0'/0/13",
          index: 13,
          address: 'yhWqJXsp25aZNQHEprebQrqPoANj6A13Aa',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/14": {
          path: "m/44'/1'/0'/0/14",
          index: 14,
          address: 'yjV3sKAGsuJHDGyf6HDMNuLfMgGp5pBxRy',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/15": {
          path: "m/44'/1'/0'/0/15",
          index: 15,
          address: 'yjPeTiRatdvotxUuPFEPDJc2aF774uMB9J',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/16": {
          path: "m/44'/1'/0'/0/16",
          index: 16,
          address: 'yRrhuVw6Vd3NzgYrfqb1oTvdhyxzDT9PGz',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/17": {
          path: "m/44'/1'/0'/0/17",
          index: 17,
          address: 'yYpL5JJLVGfJXPE15ZMQzNvUGkD4JY6ETF',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/18": {
          path: "m/44'/1'/0'/0/18",
          index: 18,
          address: 'ygcW1365Hs2LSLY5LXnkJAUB94pS4HouNu',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/0/19": {
          path: "m/44'/1'/0'/0/19",
          index: 19,
          address: 'yW8RA7zTUz14sNiGjvFQaNupugwwmE1aQi',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        }
      },
      internal: {
        "m/44'/1'/0'/1/0": {
          path: "m/44'/1'/0'/1/0",
          index: 0,
          address: 'yaLhoAZ4iex2zKmfvS9rvEmxXmRiPrjHdD',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/1": {
          path: "m/44'/1'/0'/1/1",
          index: 1,
          address: 'yTcjWB7v7opDzpfYKpFdFEtEvSKFsh3bW3',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/2": {
          path: "m/44'/1'/0'/1/2",
          index: 2,
          address: 'yiDVYtUZ2mKV4teSJzKBArqY4BRsZoFLYs',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/3": {
          path: "m/44'/1'/0'/1/3",
          index: 3,
          address: 'ya7Me5KMoSz5x4GGZ1pJGrJjC3yMkDDWDa',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/4": {
          path: "m/44'/1'/0'/1/4",
          index: 4,
          address: 'yXwS8mRrrxF3pt1GfG7yGKNpPnD6pdwX3a',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/5": {
          path: "m/44'/1'/0'/1/5",
          index: 5,
          address: 'yXd4eqycSaJRhRZxXT3iK5H34af4TV5REE',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/6": {
          path: "m/44'/1'/0'/1/6",
          index: 6,
          address: 'yWaDfpToRxHc3qtcd8P1agW4Fvj1ueWgwH',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/7": {
          path: "m/44'/1'/0'/1/7",
          index: 7,
          address: 'yPYdu2jDrD3Bai83AdvHTYwpAgAkzMaCcM',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/8": {
          path: "m/44'/1'/0'/1/8",
          index: 8,
          address: 'yMEkuZ67vZ6kUgDHVVDSTzwU3GbHoTFwqR',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/9": {
          path: "m/44'/1'/0'/1/9",
          index: 9,
          address: 'yYSSMwkEqU4hNF2z5kbVBTDYtgt8dQQYd7',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/10": {
          path: "m/44'/1'/0'/1/10",
          index: 10,
          address: 'yfzAa63gQ6arpyBzuqQtZmSnU8HLnJnEan',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/11": {
          path: "m/44'/1'/0'/1/11",
          index: 11,
          address: 'ySjTorgG6VVfPiY7TJ2tdU2hkohfFAtzJf',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/12": {
          path: "m/44'/1'/0'/1/12",
          index: 12,
          address: 'yMUJsy5HEiQTeDgqxY2zBGEeTCDr4g5V8c',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/13": {
          path: "m/44'/1'/0'/1/13",
          index: 13,
          address: 'yWDkvzhKk7BKEU6Ybz1Kyarejzt8zhSqxy',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/14": {
          path: "m/44'/1'/0'/1/14",
          index: 14,
          address: 'yTJQE4vYXRdEcPt3nADF36S6FKLkLRG2Ty',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/15": {
          path: "m/44'/1'/0'/1/15",
          index: 15,
          address: 'yTjku3TMxJN3uiiSHYB2tQ4wp1rJJ92M2A',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/16": {
          path: "m/44'/1'/0'/1/16",
          index: 16,
          address: 'yZ8JgZrpuEr1srdcgLgTAsomtMBgSYwTrW',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/17": {
          path: "m/44'/1'/0'/1/17",
          index: 17,
          address: 'yUG2YZrss5JfZrN4AG4RKkkbfYyd4H6TSx',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/18": {
          path: "m/44'/1'/0'/1/18",
          index: 18,
          address: 'yiLMqyvjBV3CqkCL8H44bRSUBap7tPCmvo',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        },
        "m/44'/1'/0'/1/19": {
          path: "m/44'/1'/0'/1/19",
          index: 19,
          address: 'yfrCYcuD7ezYnpNAmbPMjrTETjipsNGsEe',
          transactions: [],
          balanceSat: 0,
          unconfirmedBalanceSat: 0,
          utxos: {},
          fetchedLast: 0,
          used: false
        }
      },
      misc: {}
    }


  };

    const result = classifyAddresses(accountStore.addresses, accountIndex, walletType);
    const expectedResult = {
      externalAddressList: [
        'yd1ohc12LgCYp56CDuckTEHwoa6LbPghMd',
        'yMX3ycrLVF2k6YxWQbMoYgs39aeTfY4wrB',
        'ydJGUUmNxdmvyskoZXqtJRqWyqsaFPitGQ',
        'yhugNjDRVJUL7PK9MqQP9M6M1HJm8zuWJL',
        'ySgckdRYxpa7Uda8yUNRqjYeuusqLy3AY3',
        'yS4DeSU3MTBisgL6p8PDRzSxTPN2PUt3vE',
        'yRZef5UGotGgLMaLYTzhvfknogqMRBkUiX',
        'yTUw2bGzi9rYs41XH1dxbRqiJoDtwuUcv2',
        'yNQHAGhNP7UbhxnkZH4muP2oKkuayGEuwX',
        'yPPDLBDjHctWpMxLiMTJXLngcYPke7YNaY',
        'ya2vVJAJdZN2We7MYiSjGf9wkdWF6A1RLr',
        'ya5k2YMjfyfxZoidq4UdQ65jYUXvtVomEv',
        'yhXZja3Apyp9S32zEVsPqLssNJZLrczxJC',
        'yhWqJXsp25aZNQHEprebQrqPoANj6A13Aa',
        'yjV3sKAGsuJHDGyf6HDMNuLfMgGp5pBxRy',
        'yjPeTiRatdvotxUuPFEPDJc2aF774uMB9J',
        'yRrhuVw6Vd3NzgYrfqb1oTvdhyxzDT9PGz',
        'yYpL5JJLVGfJXPE15ZMQzNvUGkD4JY6ETF',
        'ygcW1365Hs2LSLY5LXnkJAUB94pS4HouNu',
        'yW8RA7zTUz14sNiGjvFQaNupugwwmE1aQi'
      ],
      internalAddressList: [
        'yaLhoAZ4iex2zKmfvS9rvEmxXmRiPrjHdD',
        'yTcjWB7v7opDzpfYKpFdFEtEvSKFsh3bW3',
        'yiDVYtUZ2mKV4teSJzKBArqY4BRsZoFLYs',
        'ya7Me5KMoSz5x4GGZ1pJGrJjC3yMkDDWDa',
        'yXwS8mRrrxF3pt1GfG7yGKNpPnD6pdwX3a',
        'yXd4eqycSaJRhRZxXT3iK5H34af4TV5REE',
        'yWaDfpToRxHc3qtcd8P1agW4Fvj1ueWgwH',
        'yPYdu2jDrD3Bai83AdvHTYwpAgAkzMaCcM',
        'yMEkuZ67vZ6kUgDHVVDSTzwU3GbHoTFwqR',
        'yYSSMwkEqU4hNF2z5kbVBTDYtgt8dQQYd7',
        'yfzAa63gQ6arpyBzuqQtZmSnU8HLnJnEan',
        'ySjTorgG6VVfPiY7TJ2tdU2hkohfFAtzJf',
        'yMUJsy5HEiQTeDgqxY2zBGEeTCDr4g5V8c',
        'yWDkvzhKk7BKEU6Ybz1Kyarejzt8zhSqxy',
        'yTJQE4vYXRdEcPt3nADF36S6FKLkLRG2Ty',
        'yTjku3TMxJN3uiiSHYB2tQ4wp1rJJ92M2A',
        'yZ8JgZrpuEr1srdcgLgTAsomtMBgSYwTrW',
        'yUG2YZrss5JfZrN4AG4RKkkbfYyd4H6TSx',
        'yiLMqyvjBV3CqkCL8H44bRSUBap7tPCmvo',
        'yfrCYcuD7ezYnpNAmbPMjrTETjipsNGsEe'
      ],
      otherAccountAddressList: []
    };
    expect(result).to.deep.equal(expectedResult);
  });
});
