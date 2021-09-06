const {expect} = require('chai');
const {Transaction, BlockHeader} = require('@dashevo/dashcore-lib');
const {WALLET_TYPES} = require('../../../CONSTANTS');
const getTransactions = require('./getTransactions');
const getTransactionHistory = require('./getTransactionHistory');
const searchTransaction = require('../../Storage/methods/searchTransaction');
const getTransaction = require('../../Storage/methods/getTransaction');
const getBlockHeader = require('../../Storage/methods/getBlockHeader');
const searchBlockHeader = require('../../Storage/methods/searchBlockHeader');
const searchAddress = require('../../Storage/methods/searchAddress');
const mockedStoreHDWallet = require('../../../../fixtures/duringdevelop-fullstore-snapshot-1548538361');
const mockedStoreSingleAddress = require('../../../../fixtures/da07-fullstore-snapshot-1548533266');


const normalizedHDStoreFixtures = require('../../../../fixtures/wallets/apart-trip-dignity/store.json');
const normalizedPKStoreFixtures = require('../../../../fixtures/wallets/2a331817b9d6bf85100ef0/store.json');
const CONSTANTS = require("../../../CONSTANTS");
const normalizedStoreToStore = (normalizedStore) => {
  const store = {
    ...normalizedStore
  };

  for (let walletId in store.wallets) {
    for (let addressType in store.wallets[walletId].addresses) {
      for (let path in store.wallets[walletId].addresses[addressType]) {
        for (let utxo in store.wallets[walletId].addresses[addressType][path].utxos) {
          store.wallets[walletId].addresses[addressType][path].utxos[utxo] = new Transaction.Output(store.wallets[walletId].addresses[addressType][path].utxos[utxo]);
        }
      }
    }
  }

  for (let transactionHash in store.transactions) {
    store.transactions[transactionHash] = new Transaction(store.transactions[transactionHash]);
  }

  for (let blockHeaderHash in store.chains['testnet'].blockHeaders) {
    store.chains['testnet'].blockHeaders[blockHeaderHash] = new BlockHeader(store.chains['testnet'].blockHeaders[blockHeaderHash])
  }
  return store;
}
const mockedSelf = {
  getTransactions,
  network: 'testnet',
  walletId: 'd6143ef4e6',
  walletType: CONSTANTS.WALLET_TYPES.HDWALLET,
  index: 0,
  storage: {
    store: {
      transactions: {},
      wallets: {
        'd6143ef4e6': {
          "addresses": {
            "external": {},
            "internal": {},
            "misc": {}
          }

        }
      },
      "chains": {
        "testnet": {
          name: "testnet",
          blockHeaders: {},
          mappedBlockHeaderHeights: {},
          blockHeight: 0
        }
      },
    },
    getStore: () => {
      return mockedSelf.storage.store;
    }
  }
}
describe('Account - getTransactionHistory', () => {
  it('should return empty array on no transaction history', async function () {
    const mockedHDSelf = {
      ...mockedSelf
    }
    const transactionHistoryHD = await getTransactionHistory.call(mockedHDSelf);
    const expectedTransactionHistoryHD = [];
    expect(transactionHistoryHD).to.deep.equal(expectedTransactionHistoryHD);
  });
  it('should return valid transaction for HDWallet', async function () {
    const mockedHDSelf = {
      ...mockedSelf
    }
    mockedHDSelf.storage.store = normalizedStoreToStore(normalizedHDStoreFixtures)
    const timestartTs = +new Date();
    const transactionHistoryHD = await getTransactionHistory.call(mockedHDSelf);
    const timeendTs = +new Date();
    const calculationTime = timeendTs - timestartTs;
    expect(calculationTime).to.be.below(60 * 1000);

    const expectedTransactionHistoryHD = [
      {
        from: [
          {address: 'yirJaK8KCE5YAmwvLadizqFw3TCXqBuZXL'},
          {address: 'yiXh4Yo5djG6QH8WzXkKm5EFzqLRJWakXz'}
        ],
        to: [
          {
            address: 'yMX3ycrLVF2k6YxWQbMoYgs39aeTfY4wrB',
            satoshis: 1000000000
          },
          {
            address: 'yhdRfg5gNr587dtEC4YYMcSHmLVEGqqtHc',
            satoshis: 159999359
          }
        ],
        type: 'sent',
        time: 1629237076,
        txId: 'e6b6f85a18d77974f376f05d6c96d0fdde990e733664248b1a00391565af6841',
        blockHash: '000001f9c5de4d2b258a975bfbf7b9a3346890af6389512bea3cb6926b9be330',
        isChainLocked: true,
        isInstantLocked: true
      },
      {
        from: [{address: 'yNCqctyQaq51WU1hN5aNwsgMsZ5fRiB7GY'}],
        to: [
          {
            address: 'yiXh4Yo5djG6QH8WzXkKm5EFzqLRJWakXz',
            satoshis: 1150000000
          },
          {
            address: 'yh6Hcyipdvp6WJpQxjNbaXP4kzPQUJpY3n',
            satoshis: 49999753
          }
        ],
        type: 'account_transfer',
        time: 1629236158,
        txId: '6f76ca8038c6cb1b373bbbf80698afdc0d638e4a223be12a4feb5fd8e1801135',
        blockHash: '000000444b3f2f02085f8befe72da5442c865c290658766cf935e1a71a4f4ba7',
        isChainLocked: true,
        isInstantLocked: true
      },
      {
        from: [{address: 'yj8rRKATAUHcAgXvNZekob58xKm2oNyvhv'}],
        to: [
          {
            address: 'yYJmzWey5kNecAThet5BFxAga1F4b4DKQ2',
            satoshis: 1260000000
          },
          {
            address: 'yirJaK8KCE5YAmwvLadizqFw3TCXqBuZXL',
            satoshis: 9999753
          }
        ],
        type: 'account_transfer',
        time: 1629234873,
        txId: '6f37b0d6284aab627c31c50e1c9d7cce39912dd4f2393f91734f794bc6408533',
        blockHash: '000000dffb05c071a8c05082a475b7ce9c1e403f3b89895a6c448fe08535a5f5',
        isChainLocked: true,
        isInstantLocked: true
      },
      {
        from: [{address: 'yj8rRKATAUHcAgXvNZekob58xKm2oNyvhv'}],
        to: [
          {
            address: 'yj8rRKATAUHcAgXvNZekob58xKm2oNyvhv',
            satoshis: 1270000000
          },
          {
            address: 'yhaAB6e8m3F8zmGX7WAVYa6eEfmSrrnY8x',
            satoshis: 400000000
          },
          {
            address: 'yLk4Hw3w4zDudrDVP6W8J9TggkY57zQUki',
            satoshis: 107099720
          }
        ],
        type: 'address_transfer',
        time: 1629234474,
        txId: 'c3fb3620ebd1c7678879b40df1495cc86a179b5a6f9e48ce0b687a5c6f5a1db5',
        blockHash: '000001953ea0bbb8ad04a9a1a2a707fef207ad22a712d7d3c619f0f9b63fa98c',
        isChainLocked: true,
        isInstantLocked: true
      },
      {
        from: [
          {address: 'ygHAVkMtYSqoTWHebDv7qkhMV6dHyuRsp2'},
          {address: 'ygk3GCSba2J3L9G665Snozhj9HSkh5ByVE'},
          {address: 'yTwEca67QSkZ6axGdpNFzWPaCj8zqYybY7'},
          {address: 'yercyhdN9oEkZcB9BsW5ktFaDxFEuK6qXN'},
          {address: 'yMLhEsiP2ajSh8STmXnNmkWXtoHsmawZxd'}
        ],
        to: [
          {
            address: 'yj8rRKATAUHcAgXvNZekob58xKm2oNyvhv',
            satoshis: 1777100000
          },
          {
            address: 'yNDpPsJqXKM36zHSNEW7c1zSvNnrZ699FY',
            satoshis: 99170
          }
        ],
        type: 'address_transfer',
        time: 1629216608,
        txId: 'f230a9414bf577d93d6f7f2515d9b549ede78cfba4168920892970fa8aa1eef8',
        blockHash: '00000084b4d9e887a6ad3f37c576a17d79c35ec9301e55210eded519e8cdcd3a',
        isChainLocked: true,
        isInstantLocked: true
      },
      {
        from: [{address: 'yP8A3cbdxRtLRduy5mXDsBnJtMzHWs6ZXr'}],
        to: [
          {
            address: 'yY16qMW4TSiYGWUyANYWMSwgwGe36KUQsR',
            satoshis: 46810176
          },
          {
            address: 'ygHAVkMtYSqoTWHebDv7qkhMV6dHyuRsp2',
            satoshis: 729210000
          }
        ],
        type: 'received',
        time: 1629207543,
        txId: '1cbb35edc105918b956838570f122d6f3a1fba2b67467e643e901d09f5f8ac1b',
        blockHash: '00000c1e4556add15119392ed36ec6af2640569409abfa23a9972bc3be1b3717',
        isChainLocked: true,
        isInstantLocked: true
      }
    ]
    expect(transactionHistoryHD).to.deep.equal(expectedTransactionHistoryHD);
  });
  it('should correctly deal with HDWallet accounts', async function () {
    const mockedHDSelf = {
      ...mockedSelf
    }
    mockedHDSelf.storage.store = normalizedStoreToStore(normalizedHDStoreFixtures)
    mockedHDSelf.index = 1;
    const transactionHistoryHD = await getTransactionHistory.call(mockedHDSelf);
    const expectedTransactionHistoryHD = [
      {
        from: [ { address: 'yNCqctyQaq51WU1hN5aNwsgMsZ5fRiB7GY' } ],
        to: [
          {
            address: 'yiXh4Yo5djG6QH8WzXkKm5EFzqLRJWakXz',
            satoshis: 1150000000
          },
          {
            address: 'yh6Hcyipdvp6WJpQxjNbaXP4kzPQUJpY3n',
            satoshis: 49999753
          }
        ],
        type: 'account_transfer',
        time: 1629236158,
        txId: '6f76ca8038c6cb1b373bbbf80698afdc0d638e4a223be12a4feb5fd8e1801135',
        blockHash: '000000444b3f2f02085f8befe72da5442c865c290658766cf935e1a71a4f4ba7',
        isChainLocked: true,
        isInstantLocked: true
      },
      {
        from: [ { address: 'yYJmzWey5kNecAThet5BFxAga1F4b4DKQ2' } ],
        to: [
          {
            address: 'yNCqctyQaq51WU1hN5aNwsgMsZ5fRiB7GY',
            satoshis: 1200000000
          },
          {
            address: 'yXMrw79LPgu78EJsfGGYpm6fXKc1EMnQ49',
            satoshis: 59999753
          }
        ],
        type: 'address_transfer',
        time: 1629235557,
        txId: '9cd3d44a87a7f99a33aebc6957105d5fb41698ef642189a36bac59ec0b5cd840',
        blockHash: '0000016fb685b4b1efed743d2263de34a9f8323ed75e732654b1b951c5cb4dde',
        isChainLocked: true,
        isInstantLocked: true
      },
      {
        from: [ { address: 'yj8rRKATAUHcAgXvNZekob58xKm2oNyvhv' } ],
        to: [
          {
            address: 'yYJmzWey5kNecAThet5BFxAga1F4b4DKQ2',
            satoshis: 1260000000
          },
          {
            address: 'yirJaK8KCE5YAmwvLadizqFw3TCXqBuZXL',
            satoshis: 9999753
          }
        ],
        type: 'account_transfer',
        time: 1629234873,
        txId: '6f37b0d6284aab627c31c50e1c9d7cce39912dd4f2393f91734f794bc6408533',
        blockHash: '000000dffb05c071a8c05082a475b7ce9c1e403f3b89895a6c448fe08535a5f5',
        isChainLocked: true,
        isInstantLocked: true
      }
    ]
    expect(transactionHistoryHD).to.deep.equal(expectedTransactionHistoryHD);
  });
  it('should correctly compute transaction history for private key based wallet', async function (){
    const mockedPKSelf = {
      ...mockedSelf
    }
    mockedPKSelf.storage.store = normalizedStoreToStore(normalizedPKStoreFixtures)
    mockedPKSelf.walletType = 'single_address';
    mockedPKSelf.walletId = '6101b44d50';

    const transactionHistoryPK = await getTransactionHistory.call(mockedPKSelf);
    const expectedTransactionHistoryPK = [
      {
        from: [ { address: 'ycDeuTfs4U77bTb5cq17dame28zdWHVYfk' } ],
        to: [
          {
            address: 'yP8A3cbdxRtLRduy5mXDsBnJtMzHWs6ZXr',
            satoshis: 450000
          },
          {
            address: 'ycDeuTfs4U77bTb5cq17dame28zdWHVYfk',
            satoshis: 8999753
          }
        ],
        type: 'address_transfer',
        time: 1629510092,
        txId: '47d13f7f713f4258953292c2298c1d91e2d6dee309d689f3c8b44ccf457bab52',
        blockHash: '0000007b7356e715b43ed7d5b7135fb9a2bf403e079bbcf7faec0f0da5c40117',
        isChainLocked: true,
        isInstantLocked: true
      },
      {
        from: [ { address: 'ycDeuTfs4U77bTb5cq17dame28zdWHVYfk' } ],
        to: [
          {
            address: 'ycDeuTfs4U77bTb5cq17dame28zdWHVYfk',
            satoshis: 9450000
          },
          {
            address: 'ycDeuTfs4U77bTb5cq17dame28zdWHVYfk',
            satoshis: 699999753
          }
        ],
        type: 'address_transfer',
        time: 1629509216,
        txId: 'd48f415f08fb795d43b216cf56e9ef10e059d4009cfc8fc90edfc0d3850813af',
        blockHash: '0000018b88fe43d07c3d63050aa82271698dc406dd08388529205dd837bf92dc',
        isChainLocked: true,
        isInstantLocked: true
      },
      {
        from: [
          { address: 'yXpVMRLKnH9e9Bdcd68e8iA3rxAerzwKop' },
          { address: 'yeryenDBwJbe7rqdL5uv7iLiJAWSU1iTe2' }
        ],
        to: [
          {
            address: 'yanVwuG1csehvH7PoWHxmYmjtojXBLnoYP',
            satoshis: 4840346
          },
          {
            address: 'ycDeuTfs4U77bTb5cq17dame28zdWHVYfk',
            satoshis: 709450000
          }
        ],
        type: 'received',
        time: 1629503698,
        txId: '0dcdaa9bf5b3596be1bcf22113e39026fd49d24b47190e2c7423be936cb116a7',
        blockHash: '000000299efeefa87dc15474fd0423c136798975b779a2bb8aa5bb2f50509afb',
        isChainLocked: true,
        isInstantLocked: true
      }
    ]

    expect(transactionHistoryPK).to.deep.equal(expectedTransactionHistoryPK);

  })
});
