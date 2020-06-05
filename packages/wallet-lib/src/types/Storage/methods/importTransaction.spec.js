const {expect} = require('chai');
const importTransaction = require('./importTransaction');
const {fd7c727155ef67fd5c1d54b73dea869e9690c439570063d6e96fec1d3bba450e} = require('../../../../fixtures/transactions').valid.mainnet;
const {Transaction} = require('@dashevo/dashcore-lib');

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
    const mockOpts1 = {};
    const mockOpts2 = '688dd18dea2b6f3c2d3892d13b41922fde7be01cd6040be9f3568dafbf9b1a23';
    const mockOpts3 = {'688dd18dea2b6f3c2d3892d13b41922fde7be01cd6040be9f3568dafbf9b1a23': {}};
    const mockOpts4 = {txid: '688dd18dea2b6f3c2d3892d13b41922fde7be01cd6040be9f3568dafbf9b1a23'};
    const mockOpts5 = {txid: '688dd18dea2b6f3c2d3892d13b41922fde7be01cd6040be9f3568dafbf9b1a23', vin: []};

    const exceptedException1 = 'A Dashcore transaction object is required';

    expect(() => importTransaction.call({}, mockOpts1)).to.throw(exceptedException1);
    expect(() => importTransaction.call({}, mockOpts2)).to.throw(exceptedException1);
    expect(() => importTransaction.call({}, mockOpts3)).to.throw(exceptedException1);
    expect(() => importTransaction.call({}, mockOpts4)).to.throw(exceptedException1);
    expect(() => importTransaction.call({}, mockOpts5)).to.throw(exceptedException1);
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
    const expectedMappedAddress =  {
      'yS3Ja63BpkH7qHYVQvdEuiBd9xo8ZoPjZB': {walletId: 'db158d08df', type: 'external', path: "m/44'/1'/0'/0/0"}
    };

    expect(self.store).to.be.deep.equal(expectedStore);
    expect(self.mappedAddress).to.be.deep.equal(expectedMappedAddress);
    expect(self.lastModified).to.be.not.equal(0);
    expect(announceCalled).to.be.equal(1);
  });
});
