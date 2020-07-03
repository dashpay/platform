const { expect } = require('chai');

const DAPIClientTransport = require('../DAPIClientTransport');

describe('transports - DAPIClientTransport - .getUTXO', function suite() {
  let fixture;
  let transport;
  let clientMock;

  beforeEach(() => {
    fixture = {
      totalItems: 1,
      from: 0,
      to: 1,
      items: [
        {
          address: 'yYpSw2n2TRzoQaUShNsPo541z4bz4EJkGN',
          txid: '3ab6ebc86b9cdea1580d376510e54a904f74fcaf38dfe9363fb44bcf33f83703',
          outputIndex: 0,
          script: '76a914891da44c4bb40cbc32a186a99bb5f935ae92750288ac',
          satoshis: 1000000000,
          height: 9484,
        },
      ],
    }

    clientMock = {
      core: {
        getUTXO: () => fixture,
      }
    }

    transport = new DAPIClientTransport(clientMock);
  })

  afterEach(() => {
    transport.disconnect();
  })

  it('should work', async () => {
    const res = await transport.getUTXO('yYpSw2n2TRzoQaUShNsPo541z4bz4EJkGN');

    expect(res).to.deep.equal(fixture.items);
  });
});
