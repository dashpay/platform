const {expect} = require('chai');
const importTransaction = require('./importTransaction');
const transactionFixtures = require('../../../../fixtures/transactions');
const {fd7c727155ef67fd5c1d54b73dea869e9690c439570063d6e96fec1d3bba450e} = transactionFixtures.valid.mainnet
const { Transaction, Script } = require('@dashevo/dashcore-lib');

const faltyTx = '03000500010000000000000000000000000000000000000000000000000000000000000000ffffffff0602cc0c028800ffffffff0200902f50090000001976a91446e502918c04a65a3830ce89cc364b0cd301793388ac00e40b54020000001976a914ecfd5aaebcbb8f4791e716e188b20d4f0183265c88ac00000000460200cc0c0000be0c7d02ff51a9d30e39873ebb953d763595565fcbe0512a04bfa25ed0455e380000000000000000000000000000000000000000000000000000000000000000';

const tx = new Transaction({
  hash: 'ea9c4066394aa09cb7ee8f3997b8dc10b999a8d709c4046f81d8bf9341ae6e5b',
  version: 3,
  inputs: [
    {
      prevTxId: '9f398515b6fc898ebf4e7b49bbfc4359b8c89f508c6cd677e53946bd86064b28',
      outputIndex: 0,
      sequenceNumber: 4294967295,
      script: '47304402205bb4f7880fb0fc13218940ba341c30e817363e5590343d28639af921b2a5f1d40220010920ae4b00bbb657f8653cb44172b8cb13447bb5105ddaf32a2845ea0666b90121025ae98eff89505fa5ff60f919ae690de638d31f4f2fcab9a9deeaf4d48eda794b',
      scriptString: '71 0x304402205bb4f7880fb0fc13218940ba341c30e817363e5590343d28639af921b2a5f1d40220010920ae4b00bbb657f8653cb44172b8cb13447bb5105ddaf32a2845ea0666b901 33 0x025ae98eff89505fa5ff60f919ae690de638d31f4f2fcab9a9deeaf4d48eda794b',
    },
    {
      prevTxId: 'b812d9345fa8ea06af1d19b935eec65824d53779db74cd325690ad1d38a82757',
      outputIndex: 0,
      sequenceNumber: 4294967295,
      script: '483045022100ea2d17ffc417e1f70c9c9ae11b7d95a07ab359c1d9d634baba145bab7b1deb0802207507296e12acc83ce038e5bbd54c46fa78b9475536f64fb313fedb978d12b73b0121025ae98eff89505fa5ff60f919ae690de638d31f4f2fcab9a9deeaf4d48eda794b',
      scriptString: '72 0x3045022100ea2d17ffc417e1f70c9c9ae11b7d95a07ab359c1d9d634baba145bab7b1deb0802207507296e12acc83ce038e5bbd54c46fa78b9475536f64fb313fedb978d12b73b01 33 0x025ae98eff89505fa5ff60f919ae690de638d31f4f2fcab9a9deeaf4d48eda794b',
    },
    {
      prevTxId: '370b7bbd5b6e0de42a95d59e3277041ac20e945ffb93f56bb6984ba42f28a2ac',
      outputIndex: 0,
      sequenceNumber: 4294967295,
      script: '47304402207926bf9176bdc88f38dde2140b2b8b0e4f331f33bb48af12c1bcce5efbb2593c022073c188d2149d5a0bfe4adff82b63d0bc62e04f2769cdcfda50a2c5e34ab7cbf60121025ae98eff89505fa5ff60f919ae690de638d31f4f2fcab9a9deeaf4d48eda794b',
      scriptString: '71 0x304402207926bf9176bdc88f38dde2140b2b8b0e4f331f33bb48af12c1bcce5efbb2593c022073c188d2149d5a0bfe4adff82b63d0bc62e04f2769cdcfda50a2c5e34ab7cbf601 33 0x025ae98eff89505fa5ff60f919ae690de638d31f4f2fcab9a9deeaf4d48eda794b',
    },
  ],
  outputs: [
    {
      satoshis: 12999997493,
      script: '76a9143ec33076ba72b36b66b7ec571dd7417abdeb76f888ac',
    },
  ],
  nLockTime: 0,
});

describe('Storage - importTransaction', function suite() {
  this.timeout(10000);
  it('should throw on failed import', () => {
    const mockStorage = {
      store:{
        transactions:{}
      }
    }
    const mockOpts1 = {};
    const mockOpts2 = '688dd18dea2b6f3c2d3892d13b41922fde7be01cd6040be9f3568dafbf9b1a23';
    const mockOpts3 = {'688dd18dea2b6f3c2d3892d13b41922fde7be01cd6040be9f3568dafbf9b1a23': {}};
    const mockOpts4 = {txid: '688dd18dea2b6f3c2d3892d13b41922fde7be01cd6040be9f3568dafbf9b1a23'};
    const mockOpts5 = {txid: '688dd18dea2b6f3c2d3892d13b41922fde7be01cd6040be9f3568dafbf9b1a23', vin: []};

    const exceptedException1 = 'A Dashcore Transaction object or valid rawTransaction is required';

    expect(() => importTransaction.call(mockStorage, mockOpts1)).to.throw(exceptedException1);
    expect(() => importTransaction.call(mockStorage, mockOpts2)).to.throw(exceptedException1);
    expect(() => importTransaction.call(mockStorage, mockOpts3)).to.throw(exceptedException1);
    expect(() => importTransaction.call(mockStorage, mockOpts4)).to.throw(exceptedException1);
    expect(() => importTransaction.call(mockStorage, mockOpts5)).to.throw(exceptedException1);
  });
  it('should import a transaction', () => {
    const mockedSearchAddress = () => ({found: false});
    let announceCalled = 0;
    const self = {
      store: {
        wallets: {
          'db158d08df': {
            addresses: {
              external: {
                "m/44'/1'/0'/0/0": {
                  path: "m/44'/1'/0'/0/0",
                  index: 0,
                  address: 'yS3Ja63BpkH7qHYVQvdEuiBd9xo8ZoPjZB',
                  transactions: [],
                  balanceSat: 0,
                  unconfirmedBalanceSat: 0,
                  utxos: {},
                  fetchedLast: 0,
                  used: false
                }
              }
            }
          }
        },
        transactions: {},
        chains: {testnet: {blockHeight: 50000}},
      },
      mappedAddress: {
        'yS3Ja63BpkH7qHYVQvdEuiBd9xo8ZoPjZB': {walletId: 'db158d08df', type: 'external', path: "m/44'/1'/0'/0/0"}
      },
      network: 'testnet',
      lastModified: 0,
      searchAddress: mockedSearchAddress,
      announce: (annType) => {
        announceCalled += 1;
        expect(annType).to.equal('FETCHED/CONFIRMED_TRANSACTION');
      },
    };
    importTransaction.call(self, tx);
    importTransaction.call(self, tx);
    const expectedStore = {
      wallets: {
        'db158d08df': {
          addresses: {
            external: {
              "m/44'/1'/0'/0/0": {
                path: "m/44'/1'/0'/0/0",
                index: 0,
                address: 'yS3Ja63BpkH7qHYVQvdEuiBd9xo8ZoPjZB',
                transactions: ["ea9c4066394aa09cb7ee8f3997b8dc10b999a8d709c4046f81d8bf9341ae6e5b"],
                balanceSat: 12999997493,
                unconfirmedBalanceSat: 0,
                utxos: {"ea9c4066394aa09cb7ee8f3997b8dc10b999a8d709c4046f81d8bf9341ae6e5b-0": tx.outputs[0]},
                fetchedLast: 0,
                used: true
              }
            }
          }
        }
      },

      transactions: {ea9c4066394aa09cb7ee8f3997b8dc10b999a8d709c4046f81d8bf9341ae6e5b: tx},
      chains: {testnet: {blockHeight: 50000}},
    };
    const expectedMappedAddress = {
      'yS3Ja63BpkH7qHYVQvdEuiBd9xo8ZoPjZB': {walletId: 'db158d08df', type: 'external', path: "m/44'/1'/0'/0/0"}
    };

    expect(self.store).to.be.deep.equal(expectedStore);
    expect(self.mappedAddress).to.be.deep.equal(expectedMappedAddress);
    expect(self.lastModified).to.be.not.equal(0);
    expect(announceCalled).to.be.equal(1);
  });
  it('should impact input and output correctly', function () {
    let announceCalled = 0;
    const tx_79fd_1 = new Transaction('0200000002de85b10c3e4e95e94597969cd7ffda3f8dc9237d36b225326fc8b24ea895039c010000006a47304402206f3b27083662213cadcc8d511f991c6cd57a45374829f32c707f99b046aaa6e8022021bfda9808d3adda06c9535edbdfe419db27ce3cb628ab0e9b1e3eeba01732c1012103a65caff6ca4c0415a3ac182dfc2a6d3a4dceb98e8b831e71501df38aa156f2c1feffffff17bed9b68cd3c1077b1776936b78a3c964bfe27d695708a196d0f35f4dcd3cef000000006a47304402200926de33076dfd2f6a0c6830ff447a9adab4e3143f7f34883e96cb3a9513f20f0220535a42cf40c5ba6095779393f8c702c913ce2f2a62d45a2cd37c56ccdbe60445012102372247aa7ef740c54fd126f3080537be5f834f0c16ba20edfe671d7c9b538c67feffffff0240782715000000001976a914e8f859254d24c98f64253a0388ba81ef2c68712788ac60f72e9e030000001976a914300dccc87c4811311c94525c7b208fc371ab654088ac1b150000');
    const tx_79fd_2 = new Transaction('0300000001853852da3974e3e1f8548256e1930781700b2f2c6bf420c0284033d61e1f4092010000006b483045022100d601a80702d3d599b338992d05f3475688a7c5febdabf372a053aaf0cfccc1d20220284410a6095a9777ca9b0f1ca56b42f536735ab39c7158682e7dba6a47cbd50c0121033807498e192fde6bfe27933365227e262e12fbfcf4d7b37ecff100228a0b04a2ffffffff0210270000000000001976a914e330440072a28e1da250fb63f3cd07e3d5a9b6cc88ac59cf2e9e030000001976a914c0c59ed9a83f91d070876941a362ed18f1b9223788ac00000000');
    const mockedSelf = {
      store: {
        wallets: {
          '79fd90175d': {
            addresses: {
              external: {
                "m/44'/1'/0'/0/0": {
                  path: "m/44'/1'/0'/0/0",
                  index: 0,
                  address: 'yQhXpFHfxk9pLyR1sPDYWZK5xqEMWbXrCd',
                  transactions: [],
                  balanceSat: 0,
                  unconfirmedBalanceSat: 0,
                  utxos: {},
                  fetchedLast: 0,
                  used: false
                },
                "m/44'/1'/0'/0/1": {
                  path: "m/44'/1'/0'/0/1",
                  index: 1,
                  address: 'yh2i4JZ51rCFLbgk6RStaGKWB5JkQeeQYr',
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
                  address: 'yikykkDREFzxM7gNjxszrw2LYmJGHfJsdv',
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
                  address: 'ydtjKwwrxsq2Czeoeqk5ULoSXvdrnKWKWR',
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
                  address: 'yZ7zwKLSwuvZyYQXU2UGRPf1qr7nRqdH7b',
                  transactions: [],
                  balanceSat: 0,
                  unconfirmedBalanceSat: 0,
                  utxos: {},
                  fetchedLast: 0,
                  used: false
                },
              }
            }
          }
        },
        transactions: {},
        chains: {testnet: {blockHeight: 50000}},
      },
      mappedAddress: {
        'yQhXpFHfxk9pLyR1sPDYWZK5xqEMWbXrCd': {walletId: '79fd90175d', type: 'external', path: "m/44'/1'/0'/0/0"},
        'ydtjKwwrxsq2Czeoeqk5ULoSXvdrnKWKWR': {walletId: '79fd90175d', type: 'internal', path: "m/44'/1'/0'/1/0"},
        'yh2i4JZ51rCFLbgk6RStaGKWB5JkQeeQYr': {walletId: '79fd90175d', type: 'external', path: "m/44'/1'/0'/0/1"},
        'yZ7zwKLSwuvZyYQXU2UGRPf1qr7nRqdH7b': {walletId: '79fd90175d', type: 'internal', path: "m/44'/1'/0'/1/1"},
        'yikykkDREFzxM7gNjxszrw2LYmJGHfJsdv': {walletId: '79fd90175d', type: 'external', path: "m/44'/1'/0'/0/2"}
      },
      network: 'testnet',
      lastModified: 0,
      announce: (annType) => {
        announceCalled += 1;
        expect(annType).to.equal('FETCHED/CONFIRMED_TRANSACTION');
      },
    };
    importTransaction.call(mockedSelf, tx_79fd_1);
    const expectedTransactionsAfterTx1 = {
      '92401f1ed6334028c020f46b2c2f0b70810793e1568254f8e1e37439da523885': tx_79fd_1
    };
    const expectedExternalStoreAfterTx1 = {
      "m/44'/1'/0'/0/0":{
        "path": "m/44'/1'/0'/0/0",
        "index": 0,
        "address": "yQhXpFHfxk9pLyR1sPDYWZK5xqEMWbXrCd",
        "transactions": ['92401f1ed6334028c020f46b2c2f0b70810793e1568254f8e1e37439da523885'],
        "balanceSat": 15538780000,
        "unconfirmedBalanceSat": 0,
        "utxos": {
          "92401f1ed6334028c020f46b2c2f0b70810793e1568254f8e1e37439da523885-1": new Transaction.Output({
            "satoshis": 15538780000,
            "script": "76a914300dccc87c4811311c94525c7b208fc371ab654088ac"
          })
        },
        "fetchedLast": 0,
        "used": true
      }
    }
    expect(mockedSelf.store.transactions).to.deep.equal(expectedTransactionsAfterTx1);
    expect(mockedSelf.store.wallets['79fd90175d'].addresses.external["m/44'/1'/0'/0/0"]).to.deep.equal(expectedExternalStoreAfterTx1["m/44'/1'/0'/0/0"]);

    // We need to ensure it do not duplicate UTXO or TXs
    importTransaction.call(mockedSelf, tx_79fd_1);
    expect(mockedSelf.store.transactions).to.deep.equal(expectedTransactionsAfterTx1);
    expect(mockedSelf.store.wallets['79fd90175d'].addresses.external["m/44'/1'/0'/0/0"]).to.deep.equal(expectedExternalStoreAfterTx1["m/44'/1'/0'/0/0"]);

    importTransaction.call(mockedSelf, tx_79fd_2);
    const expectedTransactionsAfterTx2 = {
      '92401f1ed6334028c020f46b2c2f0b70810793e1568254f8e1e37439da523885': tx_79fd_1,
      'df7792f30588150c94b68e869bcb219b55f66f7ef97d4e5c4bb48c3a6db1250e': tx_79fd_2
    };
    expect(mockedSelf.store.transactions).to.deep.equal(expectedTransactionsAfterTx2);

    const expectedExternalStoreAfterTx2 = {
      "m/44'/1'/0'/0/0":{
        "path": "m/44'/1'/0'/0/0",
        "index": 0,
        "address": "yQhXpFHfxk9pLyR1sPDYWZK5xqEMWbXrCd",
        "transactions": [
            '92401f1ed6334028c020f46b2c2f0b70810793e1568254f8e1e37439da523885',
            'df7792f30588150c94b68e869bcb219b55f66f7ef97d4e5c4bb48c3a6db1250e'
        ],
        "balanceSat": 0,
        "unconfirmedBalanceSat": 0,
        "utxos": {},
        "fetchedLast": 0,
        "used": true
      },
      "m/44'/1'/0'/0/1":{
        "path": "m/44'/1'/0'/0/1",
        "index": 1,
        "address": "yh2i4JZ51rCFLbgk6RStaGKWB5JkQeeQYr",
        "transactions": [
          'df7792f30588150c94b68e869bcb219b55f66f7ef97d4e5c4bb48c3a6db1250e'
        ],
        "balanceSat": 10000,
        "unconfirmedBalanceSat": 0,
        "utxos": {
          "df7792f30588150c94b68e869bcb219b55f66f7ef97d4e5c4bb48c3a6db1250e-0": new Transaction.Output({
            "satoshis": 10000,
            "script": '76a914e330440072a28e1da250fb63f3cd07e3d5a9b6cc88ac'
          }),
        },
        "fetchedLast": 0,
        "used": true
      },"m/44'/1'/0'/1/0":{
        "path": "m/44'/1'/0'/1/0",
        "index": 0,
        "address": "ydtjKwwrxsq2Czeoeqk5ULoSXvdrnKWKWR",
        "transactions": [
          'df7792f30588150c94b68e869bcb219b55f66f7ef97d4e5c4bb48c3a6db1250e'
        ],
        "balanceSat": 15538769753,
        "unconfirmedBalanceSat": 0,
        "utxos": {
          "df7792f30588150c94b68e869bcb219b55f66f7ef97d4e5c4bb48c3a6db1250e-1": new Transaction.Output({
            "satoshis": 15538769753,
            "script": '76a914c0c59ed9a83f91d070876941a362ed18f1b9223788ac'
          }),
        },
        "fetchedLast": 0,
        "used": true
      }
    };
    expect(mockedSelf.store.wallets['79fd90175d'].addresses.external["m/44'/1'/0'/0/0"]).to.deep.equal(expectedExternalStoreAfterTx2["m/44'/1'/0'/0/0"]);
    expect(mockedSelf.store.wallets['79fd90175d'].addresses.external["m/44'/1'/0'/0/1"]).to.deep.equal(expectedExternalStoreAfterTx2["m/44'/1'/0'/0/1"]);
    expect(mockedSelf.store.wallets['79fd90175d'].addresses.internal["m/44'/1'/0'/1/0"]).to.deep.equal(expectedExternalStoreAfterTx2["m/44'/1'/0'/1/0"]);

    importTransaction.call(mockedSelf, tx_79fd_2);
    importTransaction.call(mockedSelf, tx_79fd_1);
    importTransaction.call(mockedSelf, tx_79fd_2);
    expect(mockedSelf.store.transactions).to.deep.equal(expectedTransactionsAfterTx2);
    expect(mockedSelf.store.wallets['79fd90175d'].addresses.external["m/44'/1'/0'/0/0"]).to.deep.equal(expectedExternalStoreAfterTx2["m/44'/1'/0'/0/0"]);
    expect(mockedSelf.store.wallets['79fd90175d'].addresses.external["m/44'/1'/0'/0/1"]).to.deep.equal(expectedExternalStoreAfterTx2["m/44'/1'/0'/0/1"]);
    expect(mockedSelf.store.wallets['79fd90175d'].addresses.internal["m/44'/1'/0'/1/0"]).to.deep.equal(expectedExternalStoreAfterTx2["m/44'/1'/0'/1/0"]);

  });
  it('should import transaction metadata', function () {
    let announceCalled = 0;

    const mockedSelf = {
      store: {
        wallets: {
          '60ee3a92b6': {
            accounts:{
              "m/44'/1'/0'": {
                label: null,
                path: "m/44'/1'/0'",
                network: 'testnet',
                blockHeight: 552160
              }
            },
            network: 'testnet',
            mnemonic: null,
            type: null,
            identityIds: [],
            addresses: {
              external: {
                "m/44'/1'/0'/0/0": {
                  path: "m/44'/1'/0'/0/0",
                  index: 0,
                  address: 'yd1ohc12LgCYp56CDuckTEHwoa6LbPghMd',
                  transactions: [],
                  balanceSat: 0,
                  unconfirmedBalanceSat: 0,
                  utxos: {},
                  fetchedLast: 0,
                  used: false
                },
              },
            }
          }
        },
        transactionsMetadata: {},
        transactions: {},

        chains: {testnet: {
            mappedBlockHeaderHeights: {
              '552160': '0000019156265f85695bf62285e32408d6057406b19374a57c009afa9116396f'
            },
            blockHeight: 552160
        }},
      },
      mappedAddress: {
        'yd1ohc12LgCYp56CDuckTEHwoa6LbPghMd': {walletId: '60ee3a92b6', type: 'external', path: "m/44'/1'/0'/0/0"},
      },
      mappedTransactionsHeight: {

      },
      network: 'testnet',
      lastModified: 0,
      announce: (annType) => {
        announceCalled += 1;
        expect(annType).to.equal('FETCHED/CONFIRMED_TRANSACTION');
      },
    };

    importTransaction.call(mockedSelf, transactionFixtures.valid.testnet["1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6"], transactionFixtures.valid.testnet.metadata["1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6"]);

    expect(mockedSelf.store.transactions['1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6']).to.exist;
    const expectedTransaction = new Transaction(transactionFixtures.valid.testnet["1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6"]);
    expect(mockedSelf.store.transactions['1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6'].toString()).to.equal(expectedTransaction.toString());

    expect(mockedSelf.store.transactionsMetadata).to.exist;
    expect(mockedSelf.store.transactionsMetadata['1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6']).to.deep.equal({
      blockHash: '0000007a84abfe1d2b4201f4844bb1e59f24daf965c928281589269f281abc01',
      height: 551438,
      instantLocked: true,
      chainLocked: true
    })

    expect(mockedSelf.mappedTransactionsHeight).to.exist;
    const expectedMetadata = transactionFixtures.valid.testnet.metadata["1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6"];
    expectedMetadata.hash = '1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6';
    expect(mockedSelf.mappedTransactionsHeight['551438']).to.deep.equal([expectedMetadata]);
  });
});
