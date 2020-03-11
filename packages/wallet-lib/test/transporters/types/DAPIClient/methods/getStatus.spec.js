const { expect } = require('chai');
const transporters = require('../../../../../src/transporters');

const fixture = {
  coreVersion: 150000, protocolVersion: 70216, blocks: 9495, timeOffset: 0, connections: 16, proxy: '', difficulty: 0.001447319555790497, testnet: false, relayFee: 0.00001, errors: '', network: 'testnet',
};

describe('transporters - DAPIClient - .getStatus', () => {
  const transporter = transporters.resolve('DAPIClient');

  it('should works', async () => {
    transporter.client.getStatus = () => fixture;
    const res = await transporter.getStatus();
    expect(res).to.deep.equal(fixture);
  });
});
