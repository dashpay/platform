const { expect } = require('chai');

const DAPIClientTransport = require('../DAPIClientTransport');
const NotFoundError = require('@dashevo/dapi-client/lib/methods/errors/NotFoundError');
const GetTransactionResponse = require('@dashevo/dapi-client/lib/methods/core/getTransaction/GetTransactionResponse');

describe('transports - DAPIClientTransport .getTransaction', function suite() {
  let fixture;
  let transport;
  let clientMock;

  beforeEach(() => {
    fixture = '03000500010000000000000000000000000000000000000000000000000000000000000000ffffffff0502b924010effffffff0240c3609a010000001976a914ecfd5aaebcbb8f4791e716e188b20d4f0183265c88ac40c3609a010000001976a914ecfd5aaebcbb8f4791e716e188b20d4f0183265c88ac00000000460200b9240000476416132511031b71167f4bb7658eab5c3957d79636767f83e0e18e2b9ed7f80000000000000000000000000000000000000000000000000000000000000000';

    clientMock = {
      core: {
        getTransaction: () => {
          return new GetTransactionResponse({
            transaction: new Buffer.from(fixture, 'hex'),
            blockHash: Buffer.from('4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176', 'hex'),
            height: 42,
            confirmations: 10,
            isInstantLocked: true,
            isChainLocked: false,
          });
        },
      }
    }

    transport = new DAPIClientTransport(clientMock);
  })

  afterEach(() => {
    transport.disconnect();
  })

  it('should work', async () => {
    const res = await transport.getTransaction('2c0ee853b91b23d881f96f0128bbb5ebb90c9ef7e7bdb4eda360b0e5abf97239');
    expect(res.transaction.hash).to.equal('2c0ee853b91b23d881f96f0128bbb5ebb90c9ef7e7bdb4eda360b0e5abf97239');
    expect(res.blockHash).to.equal('4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176');
    expect(res.height).to.equal(42);
    expect(res.instantLocked).to.equal(true);
    expect(res.chainLocked).to.equal(false);
  });

  it('should return null if transaction if not found', async () => {
    clientMock.core.getTransaction = () => { throw new NotFoundError(); };

    const res = await transport.getTransaction('2c0ee853b91b23d881f96f0128bbb5ebb90c9ef7e7bdb4eda360b0e5abf97239');
    expect(res).to.equal(null);
  });
});
