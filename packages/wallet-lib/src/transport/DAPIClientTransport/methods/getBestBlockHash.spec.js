const { expect } = require('chai');

const DAPIClientTransport = require('../DAPIClientTransport');

describe('transports - DAPIClientTransport - .getBestBlockHash', function suite() {
  let fixture;
  let transport;
  let clientMock;

  beforeEach(() => {
    fixture = '0000025d24ebe65454bd51a61bab94095a6ad1df996be387e31495f764d8e2d9';

    clientMock = {
      core: {
        getBestBlockHash: () => fixture,
      }
    }

    transport = new DAPIClientTransport(clientMock);
  })

  afterEach(() => {
    transport.disconnect();
  })

  it('should work', async () => {
    const res = await transport.getBestBlockHash();

    expect(res).to.deep.equal(fixture);
  });
});
