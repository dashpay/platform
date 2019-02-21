const RpcClientMock = require('../../../../lib/test/mock/RpcClientMock');
const RpcBlockIterator = require('../../../../lib/blockchain/blockIterator/RpcBlockIterator');

describe('RpcBlockIterator', () => {
  let rpcClientMock;
  let fromBlockHeight;
  let blockIterator;

  beforeEach(function beforeEach() {
    fromBlockHeight = 1;
    rpcClientMock = new RpcClientMock(this.sinon);
    blockIterator = new RpcBlockIterator(rpcClientMock, fromBlockHeight);
  });

  it('should iterate over blocks from the blockchain', async () => {
    const obtainedBlocks = [];

    for await (const block of blockIterator) {
      obtainedBlocks.push(block);
    }

    expect(rpcClientMock.getBlockHash).to.have.been.calledOnce.and.calledWith(fromBlockHeight);
    expect(rpcClientMock.getBlock).has.callCount(rpcClientMock.blocks.length);
    expect(obtainedBlocks).to.deep.equal(rpcClientMock.blocks);
  });

  it('should iterate from beginning when "reset" method is called', async () => {
    const { value: firstBlock } = await blockIterator.next();

    blockIterator.reset();

    const { value: secondBlock } = await blockIterator.next();

    expect(firstBlock).to.equal(secondBlock);
  });

  it('should continue iteration from new block height', async () => {
    const { value: firstBlock } = await blockIterator.next();

    expect(blockIterator.getBlockHeight()).to.equal(firstBlock.height);

    blockIterator.setBlockHeight(1);

    const { value: secondBlock } = await blockIterator.next();

    expect(blockIterator.getBlockHeight()).to.equal(secondBlock.height);

    const { value: thirdBlock } = await blockIterator.next();

    expect(firstBlock).to.equal(secondBlock);

    expect(blockIterator.getBlockHeight()).to.equal(thirdBlock.height);
  });

  it('should return fromBlockHeight if there is no current block');
  it('should throw InvalidBlockHeightError if block height is out of range');
  it('should escalate an unknown error if any thrown during retrieval of the next block hash');
});
