const RpcClientMock = require('../../../lib/test/mock/RpcClientMock');
const RpcBlockIterator = require('../../../lib/blockchain/iterator/RpcBlockIterator');

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

    let done;
    let block;

    // eslint-disable-next-line no-cond-assign
    while ({ done, value: block } = await blockIterator.next()) {
      if (done) {
        break;
      }

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

  it("should emit 'block' event", async function it() {
    const blockHandlerStub = this.sinon.stub();

    blockIterator.on('block', blockHandlerStub);

    const { value: firstBlock } = await blockIterator.next();

    expect(blockHandlerStub).to.be.calledOnce();
    expect(blockHandlerStub).to.be.calledWith(firstBlock);

    const { value: secondBlock } = await blockIterator.next();

    expect(blockHandlerStub).to.be.calledTwice();
    expect(blockHandlerStub).to.be.calledWith(secondBlock);
  });
});
