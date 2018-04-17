const RpcClientMock = require('../../../lib/test/mock/RpcClientMock');
const RpcBlockIterator = require('../../../lib/blockchain/iterator/RpcBlockIterator');
const StateTransitionHeaderIterator = require('../../../lib/blockchain/iterator/StateTransitionHeaderIterator');
const STHeadersReader = require('../../../lib/blockchain/reader/STHeadersReader');
const STHeadersReaderState = require('../../../lib/blockchain/reader/STHeadersReaderState');

describe('STHeadersReader', () => {
  let rpcClientMock;
  let blockIterator;
  let stateTransitionHeaderIterator;
  let reader;

  function setWrongBlockOnCall(s, callCount) {
    const wrongBlock = Object.assign({}, rpcClientMock.blocks[3]);
    wrongBlock.previousblockhash = 'wrong';
    const rpcMock = blockIterator.rpcClient;
    rpcMock.getBlock.onCall(callCount)
      .returns(Promise.resolve({ result: wrongBlock }))
      .callThrough();

    return wrongBlock;
  }

  beforeEach(function beforeEach() {
    rpcClientMock = new RpcClientMock(this.sinon);
    blockIterator = new RpcBlockIterator(rpcClientMock);
    stateTransitionHeaderIterator = new StateTransitionHeaderIterator(blockIterator, rpcClientMock);

    const stateData = rpcClientMock.blocks.slice(1, 2);
    const readerState = new STHeadersReaderState(stateData);

    reader = new STHeadersReader(stateTransitionHeaderIterator, readerState);
  });

  it('should set blockIterator\'s block height to last block from state + 1', () => {
    expect(blockIterator.getBlockHeight(), reader.state.getLastBlock().height + 1);
  });

  it('should emit "begin", "block", "header" and "end" events', async function it() {
    const initialHeight = blockIterator.getBlockHeight();

    const beginHandlerStub = this.sinon.stub();
    const headerHandlerStub = this.sinon.stub();
    const blockHandlerStub = this.sinon.stub();
    const endHandlerStub = this.sinon.stub();

    reader.on('begin', beginHandlerStub);
    reader.on('block', blockHandlerStub);
    reader.on('header', headerHandlerStub);
    reader.on('end', endHandlerStub);

    await reader.read();

    expect(beginHandlerStub).to.be.calledOnce();
    expect(beginHandlerStub).to.be.calledWith(initialHeight);

    expect(blockHandlerStub).has.callCount(2);
    expect(blockHandlerStub).to.be.calledWith(rpcClientMock.blocks[2]);
    expect(blockHandlerStub).to.be.calledWith(rpcClientMock.blocks[3]);

    const firstTwoBlocksSTCount = rpcClientMock.blocks[0].ts.length +
      rpcClientMock.blocks[1].ts.length;
    const notSyncedST = rpcClientMock.transitionHeaders.slice(firstTwoBlocksSTCount);

    expect(headerHandlerStub).has.callCount(notSyncedST.length);
    notSyncedST.forEach((header, i) => {
      // TODO: Should be equal objects
      expect(headerHandlerStub.getCall(i).args[0].getHash()).to.be.equals(header.getHash());
    });

    expect(endHandlerStub).to.be.calledOnce();
    expect(endHandlerStub).to.be.calledWith(blockIterator.getBlockHeight());
  });

  it('should emit "wrongSequence" and read from initial block if not able to verity sequence', async function it() {
    // 3th block will be wrong on first iteration
    const wrongBlock = setWrongBlockOnCall(this.sinon, 0);

    const blockHandlerStub = this.sinon.stub();
    const wrongSequenceStub = this.sinon.stub();

    reader.on('block', blockHandlerStub);
    reader.on('wrongSequence', wrongSequenceStub);

    await reader.read();

    expect(wrongSequenceStub).to.be.calledTwice();
    [wrongBlock, rpcClientMock.blocks[1]].forEach((block, i) => {
      expect(wrongSequenceStub.getCall(i).args[0]).to.be.deep.equals(block);
    });

    expect(blockHandlerStub).has.callCount(rpcClientMock.blocks.length);
    rpcClientMock.blocks.forEach((block, i) => {
      expect(blockHandlerStub.getCall(i).args[0]).to.be.deep.equals(block);
    });
  });

  it('should emit "wrongSequence" read from previous block if blocks sequence is wrong', async function it() {
    // 4th block will be wrong on first iteration
    const wrongBlock = setWrongBlockOnCall(this.sinon, 1);

    const blockHandlerStub = this.sinon.stub();
    const wrongSequenceStub = this.sinon.stub();

    reader.on('block', blockHandlerStub);
    reader.on('wrongSequence', wrongSequenceStub);

    await reader.read();

    expect(wrongSequenceStub).to.be.calledOnce();
    expect(wrongSequenceStub).to.be.calledWith(wrongBlock);

    const iteratedBlocks = [
      rpcClientMock.blocks[2],
      rpcClientMock.blocks[2],
      rpcClientMock.blocks[3],
    ];

    expect(blockHandlerStub).has.callCount(iteratedBlocks.length);
    iteratedBlocks.forEach((block, i) => {
      expect(blockHandlerStub.getCall(i).args[0]).to.be.deep.equals(block);
    });
  });
});
