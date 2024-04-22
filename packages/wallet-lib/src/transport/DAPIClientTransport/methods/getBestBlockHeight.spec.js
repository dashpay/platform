const { expect } = require('chai');

const DAPIClientTransport = require('../DAPIClientTransport');

const getCoreChainStatus = require('../../FixtureTransport/methods/getCoreChainStatus');

describe('transports - DAPIClientTransport - .getBestBlockHeight', function suite() {
  let fixture;
  let transport;
  let clientMock;

  beforeEach(() => {
    fixture = getCoreChainStatus();

    clientMock = {
      core: {
        getCoreChainStatus: () => fixture,
      }
    }

    transport = new DAPIClientTransport(clientMock);
  })

  afterEach(() => {
    transport.disconnect();
  })

  it('should work', async () => {
    const res = await transport.getBestBlockHeight();

    expect(res).to.deep.equal(fixture.blocks);
  });
});
