const { expect } = require('chai');

const DAPIClientTransport = require('../DAPIClientTransport');

describe('transports - DAPIClientTransport .getTransaction', function suite() {
  let fixture;
  let transport;
  let clientMock;

  beforeEach(() => {
    fixture = '03000500010000000000000000000000000000000000000000000000000000000000000000ffffffff0502b924010effffffff0240c3609a010000001976a914ecfd5aaebcbb8f4791e716e188b20d4f0183265c88ac40c3609a010000001976a914ecfd5aaebcbb8f4791e716e188b20d4f0183265c88ac00000000460200b9240000476416132511031b71167f4bb7658eab5c3957d79636767f83e0e18e2b9ed7f80000000000000000000000000000000000000000000000000000000000000000';

    clientMock = {
      core: {
        getTransaction: () => new Buffer.from(fixture, 'hex'),
      }
    }

    transport = new DAPIClientTransport(clientMock);
  })

  afterEach(() => {
    transport.disconnect();
  })

  it('should work', async () => {
    const res = await transport.getTransaction('2c0ee853b91b23d881f96f0128bbb5ebb90c9ef7e7bdb4eda360b0e5abf97239');

    expect(res.hash).to.equal('2c0ee853b91b23d881f96f0128bbb5ebb90c9ef7e7bdb4eda360b0e5abf97239');
  });
});
