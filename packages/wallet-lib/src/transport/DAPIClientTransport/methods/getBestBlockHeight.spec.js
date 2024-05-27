const { expect } = require('chai');

const DAPIClientTransport = require('../DAPIClientTransport');

describe('transports - DAPIClientTransport - .getBestBlockHeight', function suite() {
  let transport;
  let clientMock;

  beforeEach(() => {
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
