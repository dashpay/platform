const EventEmitter = require('events');
const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const dirtyChai = require('dirty-chai');
const userIndex = require('../../../lib/services/userIndex');
const wait = require('../../../lib/utils/wait');

chai.use(chaiAsPromised);
chai.use(dirtyChai);
const { expect } = chai;

const rpcMock = {
  maxBlock: 3,
  async getBestBlockHeight() { return this.maxBlock; },
  async getBlockHash(blockHeight) {
    if (blockHeight > await this.getBestBlockHeight()) {
      throw new Error('Block height out of range');
    }
    return `${blockHeight}`;
  },
  async getBlockHeight(blockHash) {
    const height = Number(blockHash);
    if (height > await this.getBestBlockHeight()) {
      throw new Error('Block hash out of range');
    }
    return height;
  },
  async getBlock(blockHash) {
    const height = await this.getBlockHeight(blockHash);
    if (height > await this.getBestBlockHeight()) {
      throw new Error('Block out of range');
    }
    const isBestBlock = height >= await this.getBestBlockHeight();
    const nextBlockHash = isBestBlock ? null : await this.getBlockHash(height + 1);
    return {
      height,
      nextblockhash: nextBlockHash,
      tx: [],
    };
  },
  getUser() { throw new Error('Not found'); },
};

const zmqMock = new EventEmitter();
zmqMock.topics = { hashblock: 'hashblock' };

describe('userIndex', () => {
  it('Should not throw out of range error', async () => {
    await expect((async () => {
      userIndex.start({ dashCoreRpcClient: rpcMock, dashCoreZmqClient: zmqMock, log: console });
      await wait(10);
      zmqMock.emit(zmqMock.topics.hashblock, '4');
      await wait(10);
    })()).not.to.be.rejected();
  });
});
