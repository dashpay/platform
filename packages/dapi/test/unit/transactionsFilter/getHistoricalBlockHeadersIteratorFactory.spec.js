const chai = require('chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const { BlockHeader } = require('@dashevo/dashcore-lib');

const getHistoricalBlockHeadersIteratorFactory = require('../../../lib/grpcServer/handlers/blockheaders-stream/getHistoricalBlockHeadersIteratorFactory');

const { expect } = chai;
chai.use(dirtyChai);
chai.use(chaiAsPromised);

describe('getHistoricalBlockHeadersIteratorFactory', () => {
  let coreRpcMock;
  beforeEach(() => {
    coreRpcMock = {
      getBlock: sinon.stub(),
      getBlockHash: sinon.stub(),
      getBlockHeaders: sinon.stub(),
    };
  });

  it('should proceed straight to done if all ranges are empty', async () => {
    coreRpcMock.getBlock.resolves({ height: 1 });
    coreRpcMock.getBlockHeaders.resolves([{}]);

    sinon.stub(BlockHeader, 'fromBuffer');

    const fromBlockHash = 'fake';
    const count = 1337;

    const getHistoricalBlockHeadersIterator = getHistoricalBlockHeadersIteratorFactory(coreRpcMock);

    const blockHeadersIterator = getHistoricalBlockHeadersIterator(
      fromBlockHash,
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

    expect(coreRpcMock.getBlock.callCount).to.be.equal(1);
    expect(coreRpcMock.getBlockHash.callCount).to.be.equal(3);
    expect(coreRpcMock.getBlockHeaders.callCount).to.be.equal(3);
  });
});
