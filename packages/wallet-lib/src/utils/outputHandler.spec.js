const { expect } = require('chai');
const outputHandler = require('./outputHandler');

describe('Utils - outputHandler', () => {
  it('should works', () => {
    const outputs = [{
      amount: 9.9999,
      address: 'yeuLv2E9FGF4D9o8vphsaC2Vxoa8ZA7Efp',
      scriptPubKey: '76a914cbdb740680e713c141e9fb32e92c7d90a3f3297588ac',
    },
    {
      amount: 9.9999,
      address: 'yeuLv2E9FGF4D9o8vphsaC2Vxoa8ZA7Efp',
      scriptPubKey: '76a914cbdb740680e713c141e9fb32e92c7d90a3f3297588ac',
      type: 'P2PKH',
    }];
    const expected = [{}, {
      value: 9.9999,
      script: '76a914cbdb740680e713c141e9fb32e92c7d90a3f3297588ac',
    }];
    const result = outputHandler(outputs);
    expect(result).to.deep.equal(expected);
  });
});
