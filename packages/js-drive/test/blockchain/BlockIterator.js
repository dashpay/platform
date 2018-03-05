const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const chaiAsPromised = require('chai-as-promised');

use(chaiAsPromised);
use(sinonChai);

const BlockIterator = require('../../lib/blockchain/BlockIterator');
const WrongBlocksSequenceError = require('../../lib/blockchain/WrongBlocksSequenceError');
const getBlockFixtures = require('../../lib/test/fixtures/getBlockFixtures');

describe('BlockIterator', () => {
  let blocks;
  let rpcClientMock;
  let getBlockHashSpy;
  let getBlockSpy;

  beforeEach(function beforeEach() {
    if (!this.sinon) {
      this.sinon = sinon.sandbox.create();
    } else {
      this.sinon.restore();
    }

    blocks = getBlockFixtures();

    rpcClientMock = {
      getBlockReturnValue: null,

      getBlockHash(height, callback) {
        const block = blocks.find(b => b.height === height);
        callback(null, { result: block ? block.hash : null });
      },
      getBlock(hash, callback) {
        let block = this.getBlockReturnValue;
        if (!block) {
          block = blocks.find(b => b.hash === hash);
        }

        callback(null, { result: block });
      },
      setGetBlockReturnValue(value) {
        this.getBlockReturnValue = value;
      },
    };

    getBlockHashSpy = this.sinon.spy(rpcClientMock, 'getBlockHash');
    getBlockSpy = this.sinon.spy(rpcClientMock, 'getBlock');
  });

  it('should iterate over blocks from blockchain', async () => {
    const fromBlockHeight = 1;
    const obtainedBlocks = [];

    const blockIterator = new BlockIterator(rpcClientMock, fromBlockHeight);

    let done;
    let block;

    // eslint-disable-next-line no-cond-assign
    while ({ done, value: block } = await blockIterator.next()) {
      if (done) {
        break;
      }

      obtainedBlocks.push(block);
    }

    expect(getBlockHashSpy).to.be.calledOnce.and.calledWith(fromBlockHeight);
    expect(getBlockSpy).has.callCount(blocks.length);
    expect(obtainedBlocks).to.be.deep.equal(blocks);
  });

  it('should should throws error if blocks sequence is wrong (e.g. reorg)', async () => {
    const blockIterator = new BlockIterator(rpcClientMock);

    await blockIterator.next();

    rpcClientMock.setGetBlockReturnValue(blocks[2]);

    expect(blockIterator.next()).to.be.rejectedWith(WrongBlocksSequenceError);
  });

  it('should iterate from begging when "reset" method is called', async () => {
    const blockIterator = new BlockIterator(rpcClientMock);

    const { value: firstBlock } = await blockIterator.next();

    blockIterator.reset();

    const { value: secondBlock } = await blockIterator.next();

    expect(firstBlock).to.be.equal(secondBlock);
  });

  it('should iterate since new block height', async () => {
    const blockIterator = new BlockIterator(rpcClientMock);

    const { value: firstBlock } = await blockIterator.next();

    expect(blockIterator.getBlockHeight()).to.be.equal(firstBlock.height);

    blockIterator.setBlockHeight(1);

    const { value: secondBlock } = await blockIterator.next();

    expect(blockIterator.getBlockHeight()).to.be.equal(secondBlock.height);

    const { value: thirdBlock } = await blockIterator.next();

    expect(firstBlock).to.be.equal(secondBlock);

    expect(blockIterator.getBlockHeight()).to.be.equal(thirdBlock.height);
  });

  it('should returns current block', async () => {
    const blockIterator = new BlockIterator(rpcClientMock);

    const { value: firstBlock } = await blockIterator.next();

    expect(blockIterator.getCurrentBlock()).to.be.equal(firstBlock);

    const { value: secondBlock } = await blockIterator.next();

    expect(blockIterator.getCurrentBlock()).to.be.equal(secondBlock);

    const { value: thirdBlock } = await blockIterator.next();

    expect(blockIterator.getCurrentBlock()).to.be.equal(thirdBlock);
  });
});
