const { expect } = require('chai');
const { simpleTransactionOptimizedAccumulator } = require('../../../../src/utils/coinSelections/strategies');
const getUTXOS = require('../../../../src/Account/getUTXOS');
const duringDevelopStore = require('../../../fixtures/duringdevelop-fullstore-snapshot-1549310417');

describe('CoinSelection - Strategy - simpleDescendingAccumulator', () => {
  it('should work as expected', () => {
    const self = {
    };

    const utxosList = getUTXOS.call(Object.assign({}, {
      store: duringDevelopStore,
      getStore: () => this.store,
      walletId: '5061b8276c',
    }));

    const outputsList1e4 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1e4 }];
    const outputsList1e5 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1e5 }];
    const outputsList1e6 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1e6 }];
    const outputsList1e7 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1e7 }];
    const outputsList1e8 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1e8 }];
    const outputsList1e9 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1e9 }];
    const outputsList1e10 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1e10 }];
    const outputsList2e10 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 2e10 }];
    const outputsList6e10 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 6e10 }];
    const outputsList999e8 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 999.99998628e8 }];
    const outputsList1e11 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1e11 }];

    const expectedRes1e4 = {
      utxos: [{
        txid: '0de8c045009815ca8e7be0f461dc1569c89d8822cded87a6cecacbff2e8c6a94', outputIndex: 0, satoshis: 10000000000, scriptPubKey: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac', address: 'yRf8x9bov39e2vHtibjeG35ZNF4BCpSZGe',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 10000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 10000000000,
    };
    const expectedRes1e5 = {
      utxos: [{
        txid: '0de8c045009815ca8e7be0f461dc1569c89d8822cded87a6cecacbff2e8c6a94', outputIndex: 0, satoshis: 10000000000, scriptPubKey: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac', address: 'yRf8x9bov39e2vHtibjeG35ZNF4BCpSZGe',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 100000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 10000000000,
    };
    const expectedRes1e6 = {
      utxos: [{
        txid: '0de8c045009815ca8e7be0f461dc1569c89d8822cded87a6cecacbff2e8c6a94', outputIndex: 0, satoshis: 10000000000, scriptPubKey: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac', address: 'yRf8x9bov39e2vHtibjeG35ZNF4BCpSZGe',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1000000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 10000000000,
    };
    const expectedRes1e7 = {
      utxos: [{
        txid: '0de8c045009815ca8e7be0f461dc1569c89d8822cded87a6cecacbff2e8c6a94', outputIndex: 0, satoshis: 10000000000, scriptPubKey: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac', address: 'yRf8x9bov39e2vHtibjeG35ZNF4BCpSZGe',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 10000000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 10000000000,
    };
    const expectedRes1e8 = {
      utxos: [{
        txid: '0de8c045009815ca8e7be0f461dc1569c89d8822cded87a6cecacbff2e8c6a94', outputIndex: 0, satoshis: 10000000000, scriptPubKey: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac', address: 'yRf8x9bov39e2vHtibjeG35ZNF4BCpSZGe',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 100000000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 10000000000,
    };
    const expectedRes1e9 = {
      utxos: [{
        txid: '0de8c045009815ca8e7be0f461dc1569c89d8822cded87a6cecacbff2e8c6a94', outputIndex: 0, satoshis: 10000000000, scriptPubKey: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac', address: 'yRf8x9bov39e2vHtibjeG35ZNF4BCpSZGe',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1000000000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 10000000000,
    };
    const expectedRes1e10 = {
      utxos: [{
        txid: '0de8c045009815ca8e7be0f461dc1569c89d8822cded87a6cecacbff2e8c6a94', outputIndex: 0, satoshis: 10000000000, scriptPubKey: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac', address: 'yRf8x9bov39e2vHtibjeG35ZNF4BCpSZGe',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 10000000000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 10000000000,
    };
    const expectedRes2e10 = {
      utxos: [{
        txid: '0de8c045009815ca8e7be0f461dc1569c89d8822cded87a6cecacbff2e8c6a94', outputIndex: 0, satoshis: 10000000000, scriptPubKey: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac', address: 'yRf8x9bov39e2vHtibjeG35ZNF4BCpSZGe',
      }, {
        txid: '0de8c045009815ca8e7be0f461dc1569c89d8822cded87a6cecacbff2e8c6a94', outputIndex: 1, satoshis: 10000000000, scriptPubKey: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac', address: 'yRf8x9bov39e2vHtibjeG35ZNF4BCpSZGe',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 20000000000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 350,
      utxosValue: 20000000000,
    };
    const expectedRes6e10 = {
      utxos: [{
        txid: '0de8c045009815ca8e7be0f461dc1569c89d8822cded87a6cecacbff2e8c6a94', outputIndex: 25, satoshis: 1000, scriptPubKey: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac', address: 'yRf8x9bov39e2vHtibjeG35ZNF4BCpSZGe',
      }, {
        txid: '0de8c045009815ca8e7be0f461dc1569c89d8822cded87a6cecacbff2e8c6a94', outputIndex: 26, satoshis: 100, scriptPubKey: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac', address: 'yRf8x9bov39e2vHtibjeG35ZNF4BCpSZGe',
      }, {
        txid: '0de8c045009815ca8e7be0f461dc1569c89d8822cded87a6cecacbff2e8c6a94', outputIndex: 27, satoshis: 1088887528, scriptPubKey: '76a9144f8aa6c3e302911b8c6b0ecb0538d209c144f84988ac', address: 'yTa2L2ZJr48sbJCnYP96RwW1D4ceeCdyHS',
      }, {
        txid: '9cec6df6996accf80be685732f06040ceda23c488ec33404da3b07bbf06dd244', outputIndex: 0, satoshis: 10000000000, scriptPubKey: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac', address: 'yRf8x9bov39e2vHtibjeG35ZNF4BCpSZGe',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 60000000000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 640,
      utxosValue: 11088888628,
    };
    const expectedRes999e8 = {
      utxos: [{
        txid: '0de8c045009815ca8e7be0f461dc1569c89d8822cded87a6cecacbff2e8c6a94', outputIndex: 25, satoshis: 1000, scriptPubKey: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac', address: 'yRf8x9bov39e2vHtibjeG35ZNF4BCpSZGe',
      }, {
        txid: '0de8c045009815ca8e7be0f461dc1569c89d8822cded87a6cecacbff2e8c6a94', outputIndex: 26, satoshis: 100, scriptPubKey: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac', address: 'yRf8x9bov39e2vHtibjeG35ZNF4BCpSZGe',
      }, {
        txid: '0de8c045009815ca8e7be0f461dc1569c89d8822cded87a6cecacbff2e8c6a94', outputIndex: 27, satoshis: 1088887528, scriptPubKey: '76a9144f8aa6c3e302911b8c6b0ecb0538d209c144f84988ac', address: 'yTa2L2ZJr48sbJCnYP96RwW1D4ceeCdyHS',
      }, {
        txid: '9cec6df6996accf80be685732f06040ceda23c488ec33404da3b07bbf06dd244', outputIndex: 0, satoshis: 10000000000, scriptPubKey: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac', address: 'yRf8x9bov39e2vHtibjeG35ZNF4BCpSZGe',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 99999998628, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 640,
      utxosValue: 11088888628,
    };


    const res1e4 = simpleTransactionOptimizedAccumulator.call(self, utxosList, outputsList1e4);
    const res1e5 = simpleTransactionOptimizedAccumulator.call(self, utxosList, outputsList1e5);
    const res1e6 = simpleTransactionOptimizedAccumulator.call(self, utxosList, outputsList1e6);
    const res1e7 = simpleTransactionOptimizedAccumulator.call(self, utxosList, outputsList1e7);
    const res1e8 = simpleTransactionOptimizedAccumulator.call(self, utxosList, outputsList1e8);
    const res1e9 = simpleTransactionOptimizedAccumulator.call(self, utxosList, outputsList1e9);
    const res1e10 = simpleTransactionOptimizedAccumulator.call(self, utxosList, outputsList1e10);
    const res2e10 = simpleTransactionOptimizedAccumulator.call(self, utxosList, outputsList2e10);
    const res6e10 = simpleTransactionOptimizedAccumulator.call(self, utxosList, outputsList6e10);
    const res999e8 = simpleTransactionOptimizedAccumulator.call(self, utxosList, outputsList999e8);

    expect(res1e4).to.deep.equal(expectedRes1e4);
    expect(res1e5).to.deep.equal(expectedRes1e5);
    expect(res1e6).to.deep.equal(expectedRes1e6);
    expect(res1e7).to.deep.equal(expectedRes1e7);
    expect(res1e8).to.deep.equal(expectedRes1e8);
    expect(res1e9).to.deep.equal(expectedRes1e9);
    expect(res1e10).to.deep.equal(expectedRes1e10);
    expect(res2e10).to.deep.equal(expectedRes2e10);
    expect(res6e10).to.deep.equal(expectedRes6e10);
    expect(res999e8).to.deep.equal(expectedRes999e8);
    expect(() => simpleTransactionOptimizedAccumulator.call(self, utxosList, outputsList1e11)).to.throw(('Unsufficient utxo amount'));
  });
});
