const { expect } = require('chai');

const DAPIClientTransport = require('../DAPIClientTransport');

describe('transports - DAPIClientTransport - .getBlockchainStatus', function suite() {
  let fixture;
  let transport;
  let clientMock;

  beforeEach(() => {
    fixture = {
      coreVersion: 150000, protocolVersion: 70216, blocks: 9495, timeOffset: 0, connections: 16, proxy: '', difficulty: 0.001447319555790497, testnet: false, relayFee: 0.00001, errors: '', network: 'testnet',
    };

    clientMock = {
      core: {
        getBlockchainStatus: () => fixture,
      }
    }

    transport = new DAPIClientTransport(clientMock);
  })

  afterEach(() => {
    transport.disconnect();
  })

  it('should work', async () => {
    const res = await transport.getBlockchainStatus();

    expect(res).to.deep.equal(fixture);
  });
});
