const { expect } = require('chai');

const DAPIClientTransport = require('../DAPIClientTransport');

const getBlockchainStatus = require('../../FixtureTransport/methods/getBlockchainStatus');

describe('transports - DAPIClientTransport - .getBestBlockHeight', function suite() {
  let fixture;
  let transport;
  let clientMock;

  beforeEach(() => {
    fixture = getBlockchainStatus();

    clientMock = {
      core: {
        getBestBlockHeight: () => 1,
      }
    }

    transport = new DAPIClientTransport(clientMock);
  })

  afterEach(() => {
    transport.disconnect();
  })

  it('should work', async () => {
    const res = await transport.getBestBlockHeight();

    expect(res).to.deep.equal(1);
  });
});
