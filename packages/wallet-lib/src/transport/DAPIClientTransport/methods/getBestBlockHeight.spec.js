const { expect } = require('chai');

const DAPIClientTransport = require('../DAPIClientTransport');

const getStatus = require('../../FixtureTransport/methods/getStatus');

describe('transports - DAPIClientTransport - .getBestBlockHeight', function suite() {
  let fixture;
  let transport;
  let clientMock;

  beforeEach(() => {
    fixture = getStatus();

    clientMock = {
      core: {
        getStatus: () => fixture,
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
