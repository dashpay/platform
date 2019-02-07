const { expect } = require('chai');
const Dashcore = require('@dashevo/dashcore-lib');
const updateNetwork = require('../../src/Wallet/updateNetwork');

describe('Wallet - update network', () => {
  it('should indicate if it worked', () => {
    expect(updateNetwork.call({})).to.be.equal(false);
    expect(updateNetwork.call({}, 'testnet')).to.be.equal(false);
    const mockAccounts = [{ updateNetwork: () => null }];
    expect(updateNetwork.call({ accounts: mockAccounts }, 'testnet')).to.be.equal(true);
  });

  it('should update the network', () => {
    let called = 0;
    const self1 = {
      network: 'testnet',
      accounts: [{ updateNetwork: () => called += 1 }],
    };
    updateNetwork.call(self1, 'mainnet');
    expect(self1.network).to.equal(Dashcore.Networks.mainnet);
    updateNetwork.call(self1, 'testnet');
    expect(self1.network).to.equal(Dashcore.Networks.testnet);

    expect(called).to.equal(2);
    const self2 = {
      network: 'mainnet',
      accounts: [{ updateNetwork: () => called += 1 }],
    };
    updateNetwork.call(self2, 'testnet');
    expect(self2.network).to.equal(Dashcore.Networks.testnet);
    updateNetwork.call(self2, 'mainnet');
    expect(self2.network).to.equal(Dashcore.Networks.mainnet);
    expect(called).to.equal(4);
  });
});
