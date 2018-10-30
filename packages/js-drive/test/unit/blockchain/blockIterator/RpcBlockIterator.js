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

  it('should iterate over blocks from blockchain', async () => {
    const obtainedBlocks = [];

    for await (const block of blockIterator) {
      obtainedBlocks.push(block);
    }

    expect(rpcClientMock.getBlockHash).to.be.calledOnce.and.calledWith(fromBlockHeight);
    expect(rpcClientMock.getBlock).has.callCount(rpcClientMock.blocks.length);
    expect(obtainedBlocks).to.be.deep.equal(rpcClientMock.blocks);
  });

  it('should iterate from begging when "reset" method is called', async () => {
    const { value: firstBlock } = await blockIterator.next();

    blockIterator.reset();

    const { value: secondBlock } = await blockIterator.next();

    expect(firstBlock).to.be.equal(secondBlock);
  });

  it('should iterate since new block height', async () => {
    const { value: firstBlock } = await blockIterator.next();

    expect(blockIterator.getBlockHeight()).to.be.equal(firstBlock.height);

    blockIterator.setBlockHeight(1);

    const { value: secondBlock } = await blockIterator.next();

    expect(blockIterator.getBlockHeight()).to.be.equal(secondBlock.height);

    const { value: thirdBlock } = await blockIterator.next();

    expect(firstBlock).to.be.equal(secondBlock);

    expect(blockIterator.getBlockHeight()).to.be.equal(thirdBlock.height);
  });
});
