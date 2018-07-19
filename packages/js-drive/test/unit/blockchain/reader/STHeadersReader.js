const RpcClientMock = require('../../../../lib/test/mock/RpcClientMock');
const RpcBlockIterator = require('../../../../lib/blockchain/iterator/RpcBlockIterator');
const StateTransitionHeaderIterator = require('../../../../lib/blockchain/iterator/StateTransitionHeaderIterator');
const STHeadersReader = require('../../../../lib/blockchain/reader/STHeadersReader');
const STHeadersReaderState = require('../../../../lib/blockchain/reader/STHeadersReaderState');

describe('STHeadersReader', () => {
  let rpcClientMock;
  let blockIterator;
  let stateTransitionHeaderIterator;

  beforeEach(function beforeEach() {
    rpcClientMock = new RpcClientMock(this.sinon);
    blockIterator = new RpcBlockIterator(rpcClientMock);
    stateTransitionHeaderIterator = new StateTransitionHeaderIterator(blockIterator, rpcClientMock);
  });

  it('should set blockIterator\'s block height to last synced block from state + 1', async () => {
    const syncedBlock = rpcClientMock.blocks[1];
    const readerState = new STHeadersReaderState([syncedBlock]);
    const reader = new STHeadersReader(stateTransitionHeaderIterator, readerState);

    expect(blockIterator.getBlockHeight(), reader.state.getLastBlock().height + 1);
  });

  it('should emit "begin", "block", "header" and "end" events', async function it() {
    const syncedBlocks = [];
    const readerState = new STHeadersReaderState(syncedBlocks);
    const reader = new STHeadersReader(stateTransitionHeaderIterator, readerState);

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
    expect(beginHandlerStub).to.be.calledWith(rpcClientMock.blocks[0].height);
    expect(blockHandlerStub).has.callCount(rpcClientMock.blocks.length);

    const stHeaders = rpcClientMock.blocks.reduce((result, block) => result.concat(block.ts), []);
    expect(headerHandlerStub).has.callCount(stHeaders.length);

    rpcClientMock.transitionHeaders.forEach((header, i) => {
      const currentArg = headerHandlerStub.getCall(i).args[0].header;
      expect(currentArg.getHash()).to.be.equal(header.getHash());
    });

    expect(endHandlerStub).to.be.calledOnce();
    expect(endHandlerStub).to.be.calledWith(blockIterator.getBlockHeight());
  });

  it('should emit "reset" and read from initial block ' +
    'if previous synced block is not present for sequence verifying and ' +
    'current block height is different from initial block height', async function it() {
    const syncedBlocks = [];
    const readerState = new STHeadersReaderState(syncedBlocks);
    const reader = new STHeadersReader(stateTransitionHeaderIterator, readerState);

    const initialBlockHash = rpcClientMock.blocks[0].hash;
    const modifiedBlock = { height: blockIterator.getBlockHeight() + 10 };
    const rpcMock = blockIterator.rpcClient;
    rpcMock.getBlock.callThrough()
      .withArgs(initialBlockHash)
      .onCall(0)
      .returns(Promise.resolve({ result: modifiedBlock }));

    const blockHandlerStub = this.sinon.stub();
    const resetStub = this.sinon.stub();

    reader.on('block', blockHandlerStub);
    reader.on('reset', resetStub);

    await reader.read();

    expect(resetStub).to.be.calledOnce();

    expect(blockHandlerStub).has.callCount(rpcClientMock.blocks.length);
    rpcClientMock.blocks.forEach((block, i) => {
      expect(blockHandlerStub.getCall(i).args[0]).to.be.deep.equal(block);
    });
  });

  it('should emit "reset" and read from initial block ' +
    'if synced blocks are behind current block for sequence verifying', async function it() {
    const syncedBlockIndex = 1;
    const syncedBlock = rpcClientMock.blocks[syncedBlockIndex];
    const readerState = new STHeadersReaderState([syncedBlock]);
    const reader = new STHeadersReader(stateTransitionHeaderIterator, readerState);

    const currentBlock = { height: readerState.getLastBlock().height - 20 };
    const rpcMock = blockIterator.rpcClient;
    rpcMock.getBlock.callThrough()
      .withArgs(rpcClientMock.blocks[syncedBlockIndex + 1].hash)
      .onCall(0)
      .returns(Promise.resolve({ result: currentBlock }));

    const blockHandlerStub = this.sinon.stub();
    const resetStub = this.sinon.stub();

    reader.on('block', blockHandlerStub);
    reader.on('reset', resetStub);

    await reader.read();

    expect(resetStub).to.be.calledOnce();
    expect(blockHandlerStub).to.be.callCount(rpcClientMock.blocks.length);
  });

  it('should emit "wrongSequence" if previous block hash not equal to current block previous hash', async function it() {
    const previousBlockIndex = 1;
    const previousBlock = rpcClientMock.blocks[previousBlockIndex];
    const readerState = new STHeadersReaderState([previousBlock]);
    const reader = new STHeadersReader(stateTransitionHeaderIterator, readerState);

    const currentBlock = { previousblockhash: 'wrong' };
    const rpcMock = blockIterator.rpcClient;
    rpcMock.getBlock.callThrough()
      .withArgs(rpcClientMock.blocks[previousBlockIndex + 1].hash)
      .onCall(0)
      .returns(Promise.resolve({ result: currentBlock }));

    const blockHandlerStub = this.sinon.stub();
    const wrongSequenceStub = this.sinon.stub();

    reader.on('block', blockHandlerStub);
    reader.on('wrongSequence', wrongSequenceStub);

    await reader.read();

    expect(wrongSequenceStub).to.be.calledOnce();
    expect(wrongSequenceStub).to.be.calledWith(currentBlock);

    expect(blockHandlerStub).has.callCount(rpcClientMock.blocks.length);
    rpcClientMock.blocks.forEach((block, i) => {
      expect(blockHandlerStub.getCall(i).args[0]).to.be.deep.equal(block);
    });
  });
});
