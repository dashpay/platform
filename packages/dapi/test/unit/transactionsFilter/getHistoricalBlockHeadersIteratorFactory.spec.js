const { BlockHeader } = require('@dashevo/dashcore-lib');
const sinon = require('sinon');

const getHistoricalBlockHeadersIteratorFactory = require('../../../lib/grpcServer/handlers/blockheaders-stream/getHistoricalBlockHeadersIteratorFactory');

describe('getHistoricalBlockHeadersIteratorFactory', () => {
  let coreRpcMock;
  let blockHeaderMock;

  const chainDataProvider = { getBlockHeaders: sinon.stub() };

  beforeEach(() => {
    coreRpcMock = {
      getBlock: sinon.stub(),
      getBlockStats: sinon.stub(),
      getBlockHash: sinon.stub(),
      getBlockHeaders: sinon.stub(),
    };

    blockHeaderMock = new BlockHeader({
      version: 536870913,
      prevHash: '0000000000000000000000000000000000000000000000000000000000000000',
      merkleRoot: 'c4970326400177ce67ec582425a698b85ae03cae2b0d168e87eed697f1388e4b',
      time: 1507208925,
      timestamp: 1507208645,
      bits: 0,
      nonce: 1449878271,
    });

    chainDataProvider.getBlockHeaders.resetHistory();
  });

  it('should proceed straight to done if all ranges are empty', async () => {
    coreRpcMock.getBlockStats.resolves({ height: 1 });
    coreRpcMock.getBlockHeaders.resolves([blockHeaderMock.toBuffer().toString('hex')]);

    const fromBlockHeight = 1;
    const count = 1337;

    const getHistoricalBlockHeadersIterator = getHistoricalBlockHeadersIteratorFactory(coreRpcMock,
      chainDataProvider);

    const blockHeadersIterator = getHistoricalBlockHeadersIterator(
      fromBlockHeight,
      count,
    );

    const r1 = await blockHeadersIterator.next();
    const r2 = await blockHeadersIterator.next();
    const r3 = await blockHeadersIterator.next();
    const r4 = await blockHeadersIterator.next();

    expect(r1.done).to.be.false();
    expect(r2.done).to.be.false();
    expect(r3.done).to.be.false();
    expect(r4.done).to.be.true();

    expect(coreRpcMock.getBlockHash.callCount).to.be.equal(3);
    expect(chainDataProvider.getBlockHeaders.callCount).to.be.equal(3);
  });
});
