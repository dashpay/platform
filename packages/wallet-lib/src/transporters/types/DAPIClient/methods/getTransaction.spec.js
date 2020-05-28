const { expect } = require('chai');
const transporters = require('../../../index');

const fixture = '03000500010000000000000000000000000000000000000000000000000000000000000000ffffffff0502b924010effffffff0240c3609a010000001976a914ecfd5aaebcbb8f4791e716e188b20d4f0183265c88ac40c3609a010000001976a914ecfd5aaebcbb8f4791e716e188b20d4f0183265c88ac00000000460200b9240000476416132511031b71167f4bb7658eab5c3957d79636767f83e0e18e2b9ed7f80000000000000000000000000000000000000000000000000000000000000000';

describe('transporters - DAPIClient .getTransaction', function suite() {
  this.timeout(10000);
  const transporter = transporters.resolve('DAPIClient');

  it('should works', async () => {
    transporter.client.getTransaction = () => new Buffer.from(fixture, 'hex');
    const res = await transporter.getTransaction('2c0ee853b91b23d881f96f0128bbb5ebb90c9ef7e7bdb4eda360b0e5abf97239');
    expect(res.hash).to.equal('2c0ee853b91b23d881f96f0128bbb5ebb90c9ef7e7bdb4eda360b0e5abf97239');
  });
});
