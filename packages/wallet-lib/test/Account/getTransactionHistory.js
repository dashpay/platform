const {expect} = require('chai');
const getTransactionHistory = require('../../src/Account/getTransactionHistory');
const searchTransaction = require('../../src/Storage/searchTransaction');
const getTransaction = require('../../src/Storage/getTransaction');
const searchAddress = require('../../src/Storage/searchAddress');
const mockedStoreHDWallet = require('../fixtures/duringdevelop-fullstore-snapshot-1548538361');
const mockedStoreSingleAddress = require('../fixtures/da07-fullstore-snapshot-1548533266');

describe('Account - getTransactionHistory', () => {
  it('should return an empty array on no transaction history', async () => {

  });
  it('should return a valid transaction history for HDWallet', async () => {
    const storageHDW = {
      store: mockedStoreHDWallet,
      getStore: () => mockedStoreHDWallet,
      mappedAddress: {},
    };
    const walletIdHDW = Object.keys(mockedStoreHDWallet.wallets)[0];
    const selfHDW = Object.assign({
      walletId: walletIdHDW,
      accountIndex: 0,
      storage: storageHDW,
    });


    selfHDW.storage.searchTransaction = searchTransaction.bind(storageHDW);
    selfHDW.storage.searchAddress = searchAddress.bind(storageHDW);
    selfHDW.getTransaction = getTransaction.bind(storageHDW);
    const txHistoryHDW = await getTransactionHistory.call(selfHDW);
    const selfHDWAccount2 = Object.assign({}, selfHDW);
    selfHDWAccount2.accountIndex = 1;

    const expectedTxHistoryHDW = [
      {
        fees: 247,
        from: [
          {
            address: 'yNfUebksUc5HoSfg8gv98ruC3jUNJUM8pT',
            amount: '500',
            valueSat: 50000000000,
          },
        ],
        time: 1548538361,
        to: [{
          address: 'yd9o9jYkwB2Ba9aMtvm56YeHfCsTXyphhD',
          amount: '150',
          valueSat: 15000000000,

        }],
        type: 'moved_account',
        txid: 'dd44373d55e6e8f3a0a0cf038de6a2c750f98a2088e074f3b6de249dad704abf',
      }, {
        fees: 247,
        from: [
          {
            address: 'ygpAb9QawEwL6kej4u3r94gC4tfoLZpaLZ',
            amount: '1000',
            valueSat: 100000000000,
          },
        ],
        time: 1548506741,
        to: [{
          address: 'yNfUebksUc5HoSfg8gv98ruC3jUNJUM8pT',
          amount: '500',
          valueSat: 50000000000,
        }],
        type: 'receive',
        txid: '4493e8a39bb97d6709ca69d391b0b99f573d3aed5ce50da5f8c7626fc1cb1a7d',
      },
      {
        fees: 930,
        from: [{address: 'yQSVFizTKcPLz2V7zoZ3HkkJ7sQmb5jXAs', amount: '350', valueSat: 35000000000},
          {
            address: 'yNY6spErvvm9C8at2KQpvAfd6TPumgyETh',
            amount: '440.99997209',
            valueSat: 44099997209,

          },
          {address: 'yhnTNo6tkmr8tA4SAL8gcci1z5rPHuaoxA', amount: '125', valueSat: 12500000000},
          {
            address: 'ySVpgHLkgrrrsbaWJhW5GMHZjeSkADrsTJ',
            amount: '49.99999753',
            valueSat: 4999999753,

          },
          {
            address: 'yeLbU1At3Cp4RD7Gunic6iy6orgnoNDhEb',
            amount: '23.99999753',
            valueSat: 2399999753,

          }],
        time: 1548410546,
        to: [{
          address: 'ycyFFyWCPSWbXLZBeYppJqgvBF7bnu8BWQ',
          amount: '989.99995785',
          valueSat: 98999995785,
        }],
        txid: 'bb0c341e970418422bb94eb20d3ddb00a350907e2ef9d6247324665f78467872',
        type: 'sent',
      },
      {
        fees: 247,
        from: [{
          address: 'yU7hmdDdi9RWem64hMz3GV3i9UWHNNK2FS',
          amount: '189.99997456',
          valueSat: 18999997456,
        }],
        time: 1548409108,
        to: [{
          address: 'yNY6spErvvm9C8at2KQpvAfd6TPumgyETh',
          amount: '139.99997456',
          valueSat: 13999997456,
        }],
        txid: '5fc934fc42534dca5bea8d4f5cc5afa721dc1ce092e854050b761e3d4b757cc7',
        type: 'moved',
      },
      {
        fees: 247,
        from: [{
          address: 'yfyTKf2PaxFvND6V5pEFWpnrbcSdy3igZQ',
          amount: '324.99999753',
          valueSat: 32499999753,
        }],

        time: 1548409108,
        to: [{
          address: 'yNY6spErvvm9C8at2KQpvAfd6TPumgyETh',
          amount: '300.99999753',
          valueSat: 30099999753,
        }],
        txid: 'f093df5d83371c2f2f167399b2b27bc79d3387c7fd41575ba44881bace228bbe',
        type: 'moved',
      },
      {
        fees: 247,
        from: [
          {address: 'yRf8x9bov39e2vHtibjeG35ZNF4BCpSZGe', amount: '450', valueSat: 45000000000},
        ],
        time: 1548153589,
        to: [{
          address: 'yhnTNo6tkmr8tA4SAL8gcci1z5rPHuaoxA',
          amount: '125',
          valueSat: 12500000000,

        }],
        txid: '9a606bc71c4c87aa7735d55dc7f01047289b77945b9617615e9afc4643e14fdf',
        type: 'moved',
      },
      {
        fees: 247,
        from: [{
          address: 'yLVQ9bZBLZmmvNQk7pPCUAGaXADQ6Rhkqt',
          amount: '539.99997703',
          valueSat: 53999997703,
        }],

        time: 1548153208,
        to: [{
          address: 'yQSVFizTKcPLz2V7zoZ3HkkJ7sQmb5jXAs',
          amount: '350',
          valueSat: 35000000000,
        }],
        txid: '00131c6c3ab8fca20380c6766f414a78f05b2e1783ce2632c9469d7357305dcb',
        type: 'moved',
      },
      {
        fees: 247,
        from: [{
          address: 'ybTg1Xema7wsGHGxSMQUSNoxyYRkTMUWJd',
          amount: '989.9999795',
          valueSat: 98999997950,
        }],
        time: 1548152219,
        to: [{
          address: 'yRf8x9bov39e2vHtibjeG35ZNF4BCpSZGe',
          amount: '450',
          valueSat: 45000000000,

        }],
        txid: 'f4b0c5df91ce3bbbcf471cfbd4b024083ad66048126bd5d6732459a07e266059',
        type: 'moved',
      },
      {
        fees: 2050,
        from: [{
          address: 'yNfUebksUc5HoSfg8gv98ruC3jUNJUM8pT',
          amount: '1000',
          valueSat: 100000000000,
        }],
        time: 1548144723,
        to: [{
          address: 'yWNrA4srrAjC9DT6UCu8NgpcqwQWa35dFX',
          amount: '10',
          valueSat: 1000000000,
        }],
        txid: 'cdcf81b69629c3157f09878076bc4f544aa01477cf59915461343476772a4a84',
        type: 'sent',
      },
      {
        fees: 374,
        from: [
          {
            address: 'yhvXpqQjfN9S4j5mBKbxeGxiETJrrLETg5',
            amount: '1100',
            valueSat: 110000000000,

          },
        ],
        time: 1548141724,
        to: [{
          address: 'yNfUebksUc5HoSfg8gv98ruC3jUNJUM8pT',
          amount: '1000',
          valueSat: 100000000000,
        }],
        type: 'receive',
        txid: '507e56181d03ba75b133f93cd073703c5c514f623f30e4cc32144c62b5a697c4',
      },
    ];

    expect(txHistoryHDW).to.be.deep.equal(expectedTxHistoryHDW);
  });
  it('should return a valid transaction history for HDWallet on secondary account', async () => {
    const storageHDW = {
      store: mockedStoreHDWallet,
      getStore: () => mockedStoreHDWallet,
      mappedAddress: {},
    };
    const walletIdHDW = Object.keys(mockedStoreHDWallet.wallets)[0];
    const selfHDW = Object.assign({
      walletId: walletIdHDW,
      accountIndex: 1,
      storage: storageHDW,
    });

    selfHDW.storage.searchTransaction = searchTransaction.bind(storageHDW);
    selfHDW.storage.searchAddress = searchAddress.bind(storageHDW);
    selfHDW.getTransaction = getTransaction.bind(storageHDW);
    const txHistoryHDW = await getTransactionHistory.call(selfHDW);


    const expectedTxHistoryHDWAccount = [
      {
        fees: 247,
        from: [
          {
            address: 'yNfUebksUc5HoSfg8gv98ruC3jUNJUM8pT',
            amount: '500',
            valueSat: 50000000000,
          },
        ],
        time: 1548538361,
        to: [{
          address: 'yd9o9jYkwB2Ba9aMtvm56YeHfCsTXyphhD',
          amount: '150',
          valueSat: 15000000000,
        }],
        type: 'moved_account',
        txid: 'dd44373d55e6e8f3a0a0cf038de6a2c750f98a2088e074f3b6de249dad704abf',
      },
    ];
    expect(txHistoryHDW).to.be.deep.equal(expectedTxHistoryHDWAccount);
  });
  it('should return a valid transaction history for SingleAddress', async () => {
    const storageSA = {
      store: mockedStoreSingleAddress,
      getStore: () => mockedStoreSingleAddress,
      mappedAddress: {},
    };
    const walletIdSA = Object.keys(mockedStoreSingleAddress.wallets)[0];
    const selfSA = Object.assign({
      walletId: walletIdSA,
      accountId: 0,
      storage: storageSA,
    });
    selfSA.storage.searchTransaction = searchTransaction.bind(storageSA);
    selfSA.storage.searchAddress = searchAddress.bind(storageSA);
    selfSA.getTransaction = getTransaction.bind(storageSA);
    const txHistorySA = await getTransactionHistory.call(selfSA);
    const expectedTxHistorySA = [
      {
        fees: 247,
        from: [
          {
            address: 'ygpAb9QawEwL6kej4u3r94gC4tfoLZpaLZ',
            amount: '499.99999753',
            valueSat: 49999999753,

          },
        ],
        time: 1548533266,
        to: [{
          address: 'ygpAb9QawEwL6kej4u3r94gC4tfoLZpaLZ',
          amount: '499.99999506',
          valueSat: 49999999506,
        }],
        type: 'moved',
        txid: '2d1cce77517e8411c9c9548884029edabbbd11ca3d13d6e11acdd90a79bb4408',
      },
      {
        fees: 247,
        from: [
          {
            address: 'ygpAb9QawEwL6kej4u3r94gC4tfoLZpaLZ',
            amount: '1000',
            valueSat: 100000000000,
          },
        ],
        time: 1548506741,
        to: [{
          address: 'yNfUebksUc5HoSfg8gv98ruC3jUNJUM8pT',
          amount: '500',
          valueSat: 50000000000,
        }],
        type: 'sent',
        txid: '4493e8a39bb97d6709ca69d391b0b99f573d3aed5ce50da5f8c7626fc1cb1a7d',
      },
      {
        fees: 522,
        from: [
          {address: 'yhvXpqQjfN9S4j5mBKbxeGxiETJrrLETg5', amount: '1000', valueSat: 100000000000},
          {address: 'yifmFokBdParjkfZp3Bu5oR9gTtHtPEU3b', amount: '2.99998582', valueSat: 299998582},
        ],
        time: 1548505964,
        to: [{
          address: 'ygpAb9QawEwL6kej4u3r94gC4tfoLZpaLZ',
          amount: '1000',
          valueSat: 100000000000,
        }],
        type: 'receive',
        txid: 'f59ea94b2edf9b42e97027cc528b10d4874ce9ff604f095072e924611463053e',
      },
    ];
    expect(txHistorySA).to.be.deep.equal(expectedTxHistorySA);
  });
  it('should output in less than a second', async () => {
    const storageHDW = {
      store: mockedStoreHDWallet,
      getStore: () => mockedStoreHDWallet,
      mappedAddress: {},
    };
    const walletIdHDW = Object.keys(mockedStoreHDWallet.wallets)[0];
    const selfHDW = Object.assign({
      walletId: walletIdHDW,
      accountIndex: 0,
      storage: storageHDW,
    });


    selfHDW.storage.searchTransaction = searchTransaction.bind(storageHDW);
    selfHDW.storage.searchAddress = searchAddress.bind(storageHDW);
    selfHDW.getTransaction = getTransaction.bind(storageHDW);
    const timestartTs = +new Date();
    const txHistoryHDW = await getTransactionHistory.call(selfHDW);
    const timeendTs = +new Date();
    const calculationTime = timeendTs - timestartTs;
    expect(calculationTime).to.be.below(60 * 1000);
  });
});
