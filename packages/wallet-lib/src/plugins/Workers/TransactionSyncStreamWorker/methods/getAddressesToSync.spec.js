const { expect } = require('chai');
const getAddressesToSync = require('./getAddressesToSync');
const KeyChainStore = require("../../../../types/KeyChainStore/KeyChainStore");
const KeyChain = require("../../../../types/KeyChain/KeyChain");
const { HDPrivateKey, HDPublicKey, PrivateKey } = require("@dashevo/dashcore-lib");


const privateKey = new PrivateKey('ee56be968a42e58fda23b83da17f90e002cafbe35a702c2f5598b13fdaa238db', 'testnet')
const hdprivateKey1 = new HDPrivateKey("xprv9s21ZrQH143K39R9Ux28kCBUHcQFdBeVE2CXFVz6GnA2a6pqTsPhHR5QHtMP5ZTRpYkKqc9ifjkJ2V1h318qWsYgyxCBUurRdTNthjgwKMw", 'testnet');
const hdpublicKey1 = new HDPublicKey("xpub661MyMwAqRbcFhaucFQun3ivEyA5gy5NKnjr1xMUVkyqdF3VNNy3TLinwnYMSUye5FF5pDSrn2SPX3zvKRQGrpZ44VVUBeuxuzov7enWpkf",'testnet');

const keychainPrivate1 = new KeyChain({privateKey});
const keychainHDPrivate1 = new KeyChain({HDPrivateKey: hdprivateKey1});
const keychainHDPublic1 = new KeyChain({HDPublicKey: hdpublicKey1});

const keychainStorePrivateKeyWallet = new KeyChainStore();
keychainStorePrivateKeyWallet.addKeyChain(keychainPrivate1, { isMasterKeyChain: true});
keychainPrivate1.getForPath(0, { isWatched: true })

const keychainStoreHDPrivateKeyWallet = new KeyChainStore();
keychainStoreHDPrivateKeyWallet.addKeyChain(keychainHDPrivate1, { isMasterKeyChain: true});
keychainHDPrivate1.getForPath(`m/0/0`, { isWatched: true })
keychainHDPrivate1.getForPath(`m/0/1`, { isWatched: true })
const mockedStore1 = {
  wallets: {
    123456789: {
      addresses: {
        misc:{
          '0':{
            address: 'yizmJb63ygipuJaRgYtpWCV2erQodmaZt1',
            balanceSat: 0,
            fetchedLast: 0,
            path: "0",
            transactions: [],
            index: 0,
            unconfirmedBalanceSat: 0,
            used: false,
            utxos: {}
          }
        }
      },
    },
  },
}
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
            utxos: {
              "dd7afaadedb5f022cec6e33f1c8520aac897df152bd9f876842f3723ab9614bc-0":
                  {
                    address: 'yizmJb63ygipuJaRgYtpWCV2erQodmaZt8',
                    txId: 'dd7afaadedb5f022cec6e33f1c8520aac897df152bd9f876842f3723ab9614bc',
                    outputIndex: 0,
                    script: '76a914f8c2652847720ab6d401291e5a48e2c8fe5d3c9f88ac',
                    satoshis: 100000000,
                  },
            }
          },
          "m/44'/1'/1'/0/0":{
            address: 'yQ5TfKcj3NHM4V4K5VBgoFJj9Q4LKX13gn',
            balanceSat: 14419880000,
            fetchedLast: 0,
            path: "m/44'/1'/1'/0/0",
            transactions: [
              'b8838022a663ae486192cf2499f9ae657e8c3a7e823a447b8b7e3d348d3916ba',
            ],
            index: 0,
            unconfirmedBalanceSat: 0,
            used: true,
            utxos: {
              "b8838022a663ae486192cf2499f9ae657e8c3a7e823a447b8b7e3d348d3916ba-0":
                  {
                    address: 'yQ5TfKcj3NHM4V4K5VBgoFJj9Q4LKX13gn',
                    txId: 'b8838022a663ae486192cf2499f9ae657e8c3a7e823a447b8b7e3d348d3916ba',
                    outputIndex: 0,
                    script: '76a914293b5b9a2154a0e4543027d694276cd5fdcb74cd88ac',
                    satoshis: 14419880000,
                  },
            }
          }
        },
        internal:{
          "m/44'/1'/0'/1/0": {
            address: 'yizmJb63ygipuJaRgYtpWCV2erQodmaZt9',
            balanceSat: 0,
            fetchedLast: 0,
            path: "m/44'/1'/0'/1/0",
            transactions: [],
            index: 0,
            unconfirmedBalanceSat: 0,
            used: false,
            utxos: {}
          }
        },
        misc:{
          '0':{
            address: 'yizmJb63ygipuJaRgYtpWCV2erQodmaZt1',
            balanceSat: 0,
            fetchedLast: 0,
            path: "0",
            transactions: [],
            index: 0,
            unconfirmedBalanceSat: 0,
            used: false,
            utxos: {}
          }
        }
      },
    },
  },
};

const mockSelfPrivateKeyType = {
  storage: { getStore:()=>mockedStore1 },
  keyChainStore: keychainStorePrivateKeyWallet,
  walletId: '123456789',
  walletType: 'privateKey',
}
const mockSelfIndex0 = {
  storage: { getStore:()=>mockedStore2 },
  keyChainStore: keychainStorePrivateKeyWallet,
  walletId: '123456789',
  walletType: 'hdwallet',
  BIP44PATH: `m/44'/1'/0'`
}
const mockSelfIndex1 = {
  ...mockSelfIndex0,
  BIP44PATH: `m/44'/1'/1'`
}


describe('TransactionSyncStreamWorker#getAddressesToSync', function suite() {
  it('should correctly fetch addresses to sync', async () => {

    const addressesIndex0 = getAddressesToSync.call(mockSelfIndex0);
    expect(addressesIndex0).to.deep.equal([
      'Xpkr9M3DP8RgcWw4SHUW75PYtmU1Lh5Ss2',
      'Xp1kwhXoUVHKRKmoXt3dB4i4KhryHSYjtW'
    ])

    const addressesIndex2 = getAddressesToSync.call(mockSelfPrivateKeyType );
    expect(addressesIndex2).to.deep.equal(['yZprpQkn7FYUHjqm3dY4sCs9SorMCi4oyR'])
  });
});
