const { expect } = require('chai');
const updateTransaction = require('../../../../src/types/Storage/methods/updateTransaction');
const orangeWStore = require('../../../fixtures/walletStore').valid.orange.store;
const { Transaction } = require('@dashevo/dashcore-lib');

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
});
describe('Storage - updateTransaction', () => {
  it('should throw on failed update', () => {
    const exceptedException1 = 'Expected a transaction to update';
    const self = {
      store: {
        transactions: {},
      },
    };

    expect(() => updateTransaction.call(self, null)).to.throw(exceptedException1);
  });
  it('should work', () => {
    const self = {
      store: {
        transactions: {
          ea9c4066394aa09cb7ee8f3997b8dc10b999a8d709c4046f81d8bf9341ae6e5b: tx,
        },
      },
    };
    const txObj = new Transaction(tx);
    txObj.nLockTime = 0;
    const expected = {
      ea9c4066394aa09cb7ee8f3997b8dc10b999a8d709c4046f81d8bf9341ae6e5b: txObj,
    };


    const update = updateTransaction.call(self, txObj);
    expect(update).to.equal(true);
    expect(self.store.transactions).to.deep.equal(expected);
  });
});
