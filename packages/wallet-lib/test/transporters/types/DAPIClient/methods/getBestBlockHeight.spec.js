const { expect } = require('chai');
const transporters = require('../../../../../src/transporters');

// const fixture = {'hash':'0000025d24ebe65454bd51a61bab94095a6ad1df996be387e31495f764d8e2d9', height: 4603};

const fixture = {
  coreVersion: 150000, protocolVersion: 70216, blocks: 9495, timeOffset: 0, connections: 16, proxy: '', difficulty: 0.001447319555790497, testnet: false, relayFee: 0.00001, errors: '', network: 'testnet',
};
describe('transporters - DAPIClient - .getBestBlockHeight', () => {
  const transporter = transporters.resolve('DAPIClient');

  it('should works', async () => {
    transporter.client.getStatus = () => fixture;
    const res = await transporter.getBestBlockHeight();
    expect(res).to.deep.equal(9495);
  });
});
