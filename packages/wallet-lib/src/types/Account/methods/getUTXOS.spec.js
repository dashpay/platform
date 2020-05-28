const { expect } = require('chai');
const Dashcore = require('@dashevo/dashcore-lib');
const getUTXOS = require('./getUTXOS');

const mockedStoreEmpty = {
  wallets: {
    123456789: {
      addresses: {},
    },
  },
};

const mockedStore2 = {
  wallets: {
    123456789: {
      addresses: {
        external: {
          "m/44'/1'/0'/0/0": {
            address: 'yizmJb63ygipuJaRgYtpWCV2erQodmaZt8',
            balanceSat: 100000000,
            fetchedLast: 0,
            path: "m/44'/1'/0'/0/0",
            transactions: [
              'dd7afaadedb5f022cec6e33f1c8520aac897df152bd9f876842f3723ab9614bc',
              '1d8f924bef2e24d945d7de2ac66e98c8625e4cefeee4e07db2ea334ce17f9c35',
              '7ae825f4ecccd1e04e6c123e0c55d236c79cd04c6ab64e839aed2ae0af3003e6',
            ],
            index: 0,
            unconfirmedBalanceSat: 0,
            used: true,
            utxos: [
              {
                address: 'yizmJb63ygipuJaRgYtpWCV2erQodmaZt8',
                txid: 'dd7afaadedb5f022cec6e33f1c8520aac897df152bd9f876842f3723ab9614bc',
                outputIndex: 0,
                scriptPubKey: '76a914f8c2652847720ab6d401291e5a48e2c8fe5d3c9f88ac',
                satoshis: 100000000,
              },
            ],
          },
        },
      },
    },
  },
};

describe('Account - getUTXOS', function suite() {
  this.timeout(10000);
  it('should get the proper UTXOS list', () => {
    const utxos = getUTXOS.call({
      store: mockedStoreEmpty,
      getStore: mockedStoreEmpty,
      walletId: '123456789',
    });
    expect(utxos).to.be.deep.equal([]);

    const utxos2 = getUTXOS.call({
      store: mockedStore2,
      getStore: mockedStore2,
      walletId: '123456789',
    });

    expect(utxos2).to.be.deep.equal([
      {
        address: 'yizmJb63ygipuJaRgYtpWCV2erQodmaZt8',
        txid: 'dd7afaadedb5f022cec6e33f1c8520aac897df152bd9f876842f3723ab9614bc',
        outputIndex: 0,
        scriptPubKey: '76a914f8c2652847720ab6d401291e5a48e2c8fe5d3c9f88ac',
        satoshis: 100000000,
      },
    ]);
  });
});
