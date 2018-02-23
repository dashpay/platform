/* eslint-disable import/no-extraneous-dependencies,no-cond-assign,no-await-in-loop */

const fs = require('fs');
const path = require('path');

const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');

use(sinonChai);

const BlockIterator = require('../../lib/blockchain/BlockIterator');

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

    const blocksJSON = fs.readFileSync(path.join(__dirname, '/../fixtures/blocks.json'));
    blocks = JSON.parse(blocksJSON);

    rpcClientMock = {
      getBlockHash(height, callback) {
        callback(null, { result: blocks[0].hash });
      },
      getBlock(hash, callback) {
        callback(null, { result: blocks.find(block => block.hash === hash) });
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
});
