const { expect } = require('chai');
const transporters = require('../../../index');

const fixture = '0000002008f7ac5b0e2df33ac233fef59549075ed24aa893ffc1d7b7067256da420000006670782820f19b64f011c55815c9315946573ac92bd5cce6deda684edcba1472c1904e5eae0d021e953d00000103000500010000000000000000000000000000000000000000000000000000000000000000ffffffff050238180101ffffffff0240c3609a010000001976a914ecfd5aaebcbb8f4791e716e188b20d4f0183265c88ac40c3609a010000001976a914ecfd5aaebcbb8f4791e716e188b20d4f0183265c88ac0000000046020038180000476416132511031b71167f4bb7658eab5c3957d79636767f83e0e18e2b9ed7f80000000000000000000000000000000000000000000000000000000000000000';

describe('transporters - DAPIClientWrapper .getBlockHeaderByHash', function suite() {
  this.timeout(10000);
  const transporter = transporters.resolve('dapi');
  it('should works', async () => {
    transporter.client.getBlockByHeight = () => new Buffer.from(fixture, 'hex');
    const res = await transporter.getBlockHeaderByHeight(6200);
    expect(res.hash).to.equal('000000c33ad38337e9bf648842f3cc08b146739d561ce468bd373ee815595436');
    expect(res.nonce).to.equal(15765);
    expect(res.timestamp).to.equal(1582207169);
  });
});
