const { expect } = require('chai');
const { simpleDescendingAccumulator } = require('../../../../src/utils/coinSelections/strategies');
const getUTXOS = require('../../../../src/types/Account/methods/getUTXOS');
const duringDevelopStore = require('../../../fixtures/duringdevelop-fullstore-snapshot-1549310417');

describe('CoinSelection - Strategy - simpleDescendingAccumulator', () => {
  it('should work as expected', () => {
    const self = {
    };

    const utxosList = getUTXOS.call({
      store: duringDevelopStore,
      getStore: () => this.store,
      walletId: '5061b8276c',
    });

    const outputsList1e4 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1e4 }];
    const outputsList1e5 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1e5 }];
    const outputsList1e6 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1e6 }];
    const outputsList1e7 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1e7 }];
    const outputsList1e8 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1e8 }];
    const outputsList1e9 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1e9 }];
    const outputsList1e10 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1e10 }];
    const outputsList2e10 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 2e10 }];
    const outputsList6e10 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 6e10 }];
    const outputsList1e11 = [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1e11 }];

    const expectedRes1e4 = {
      utxos: [{
        satoshis: 10000000000, script: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 10000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 10000000000,
    };
    const expectedRes1e5 = {
      utxos: [{
        satoshis: 10000000000, script: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 100000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 10000000000,
    };
    const expectedRes1e6 = {
      utxos: [{
        satoshis: 10000000000, script: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1000000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 10000000000,
    };
    const expectedRes1e7 = {
      utxos: [{
        satoshis: 10000000000, script: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 10000000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 10000000000,
    };
    const expectedRes1e8 = {
      utxos: [{
        satoshis: 10000000000, script: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 100000000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 10000000000,
    };
    const expectedRes1e9 = {
      utxos: [{
        satoshis: 10000000000, script: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 1000000000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 10000000000,
    };
    const expectedRes1e10 = {
      utxos: [{
        satoshis: 10000000000, script: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 10000000000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 10000000000,
    };
    const expectedRes2e10 = {
      utxos: [{
        satoshis: 10000000000, script: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac',
      }, {
        satoshis: 10000000000, script: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 20000000000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 350,
      utxosValue: 20000000000,
    };
    const expectedRes6e10 = {
      utxos: [{
        satoshis: 10000000000, script: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac',
      }, {
        satoshis: 10000000000, script: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac',
      }, {
        satoshis: 10000000000, script: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac',
      }, {
        satoshis: 10000000000, script: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac',
      }, {
        satoshis: 10000000000, script: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac',
      }, {
        satoshis: 10000000000, script: '76a9143a9202121ee9ef906e567101326f2ecf8ad4ecbc88ac',
      }],
      outputs: [{ address: 'yU7sNM4j6fzKtbah24gCXdN636piQN8F2f', satoshis: 60000000000, scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 930,
      utxosValue: 60000000000,
    };


    const res1e4 = simpleDescendingAccumulator.call(self, utxosList, outputsList1e4);
    const res1e5 = simpleDescendingAccumulator.call(self, utxosList, outputsList1e5);
    const res1e6 = simpleDescendingAccumulator.call(self, utxosList, outputsList1e6);
    const res1e7 = simpleDescendingAccumulator.call(self, utxosList, outputsList1e7);
    const res1e8 = simpleDescendingAccumulator.call(self, utxosList, outputsList1e8);
    const res1e9 = simpleDescendingAccumulator.call(self, utxosList, outputsList1e9);
    const res1e10 = simpleDescendingAccumulator.call(self, utxosList, outputsList1e10);
    const res2e10 = simpleDescendingAccumulator.call(self, utxosList, outputsList2e10);
    const res6e10 = simpleDescendingAccumulator.call(self, utxosList, outputsList6e10);

    res1e4.utxos[0] = res1e4.utxos[0].toJSON();
    expect(res1e4).to.deep.equal(expectedRes1e4);

    res1e5.utxos[0] = res1e5.utxos[0].toJSON();
    expect(res1e5).to.deep.equal(expectedRes1e5);

    res1e6.utxos[0] = res1e6.utxos[0].toJSON();
    expect(res1e6).to.deep.equal(expectedRes1e6);

    res1e7.utxos[0] = res1e7.utxos[0].toJSON();
    expect(res1e7).to.deep.equal(expectedRes1e7);

    res1e8.utxos[0] = res1e8.utxos[0].toJSON();
    expect(res1e8).to.deep.equal(expectedRes1e8);

    res1e9.utxos[0] = res1e9.utxos[0].toJSON();
    expect(res1e9).to.deep.equal(expectedRes1e9);

    res1e10.utxos[0] = res1e10.utxos[0].toJSON();
    expect(res1e10).to.deep.equal(expectedRes1e10);

    res2e10.utxos[0] = res2e10.utxos[0].toJSON();
    res2e10.utxos[1] = res2e10.utxos[1].toJSON();
    expect(res2e10).to.deep.equal(expectedRes2e10);

    res6e10.utxos[0] = res6e10.utxos[0].toJSON();
    res6e10.utxos[1] = res6e10.utxos[1].toJSON();
    res6e10.utxos[2] = res6e10.utxos[2].toJSON();
    res6e10.utxos[3] = res6e10.utxos[3].toJSON();
    res6e10.utxos[4] = res6e10.utxos[4].toJSON();
    res6e10.utxos[5] = res6e10.utxos[5].toJSON();
    expect(res6e10).to.deep.equal(expectedRes6e10);
    expect(() => simpleDescendingAccumulator.call(self, utxosList, outputsList1e11)).to.throw(('Unsufficient utxo amount'));


    // expect(res1).to.deep.equal(expectedRes1);
  });
});
