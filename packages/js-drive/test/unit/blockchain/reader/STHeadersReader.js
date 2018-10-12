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

    reader.on(STHeadersReader.EVENTS.BEGIN, beginHandlerStub);
    reader.on(STHeadersReader.EVENTS.BLOCK, blockHandlerStub);
    reader.on(STHeadersReader.EVENTS.HEADER, headerHandlerStub);
    reader.on(STHeadersReader.EVENTS.END, endHandlerStub);

    await reader.read();

    expect(beginHandlerStub).to.be.calledOnce();
    expect(beginHandlerStub).to.be.calledWith(rpcClientMock.blocks[0].height);

    expect(blockHandlerStub).has.callCount(rpcClientMock.blocks.length);

    const stHeaders = rpcClientMock.blocks.reduce((result, block) => result.concat(block.tx), []);
    expect(headerHandlerStub).has.callCount(stHeaders.length);

    rpcClientMock.transitionHeaders.forEach((header, i) => {
      const currentArg = headerHandlerStub.getCall(i).args[0].header;
      expect(currentArg.hash).to.be.equal(header.hash);
    });

    expect(endHandlerStub).to.be.calledOnce();
    expect(endHandlerStub).to.be.calledWith(blockIterator.getBlockHeight());
  });

  it('should emit "reset" and read from initial block '
    + 'if previous synced block is not present for sequence verifying and '
    + 'current block height is different from initial block height', async function it() {
    const syncedBlocks = [];
    const readerState = new STHeadersReaderState(syncedBlocks);
    const reader = new STHeadersReader(stateTransitionHeaderIterator, readerState);

    const initialBlockHash = rpcClientMock.blocks[0].hash;
    const modifiedBlock = { height: blockIterator.getBlockHeight() + 10 };
    rpcClientMock.getBlock.callThrough()
      .withArgs(initialBlockHash)
      .onCall(0)
      .returns(Promise.resolve({ result: modifiedBlock }));

    const blockHandlerStub = this.sinon.stub();
    const resetHandlerStub = this.sinon.stub();

    reader.on(STHeadersReader.EVENTS.BLOCK, blockHandlerStub);
    reader.on(STHeadersReader.EVENTS.RESET, resetHandlerStub);

    await reader.read();

    expect(resetHandlerStub).to.be.calledOnce();

    expect(blockHandlerStub).has.callCount(rpcClientMock.blocks.length);
    rpcClientMock.blocks.forEach((block, i) => {
      expect(blockHandlerStub.getCall(i).args[0]).to.be.deep.equal(block);
    });
  });

  it('should emit "reset" and read from initial block '
    + 'if synced blocks are behind current block for sequence verifying', async function it() {
    const syncedBlockIndex = 1;
    const syncedBlock = rpcClientMock.blocks[syncedBlockIndex];
    const readerState = new STHeadersReaderState([syncedBlock]);
    const reader = new STHeadersReader(stateTransitionHeaderIterator, readerState);

    const currentBlock = { height: readerState.getLastBlock().height - 20 };
    rpcClientMock.getBlock.callThrough()
      .withArgs(rpcClientMock.blocks[syncedBlockIndex + 1].hash)
      .onCall(0)
      .returns(Promise.resolve({ result: currentBlock }));

    const blockHandlerStub = this.sinon.stub();
    const resetHandlerStub = this.sinon.stub();

    reader.on(STHeadersReader.EVENTS.BLOCK, blockHandlerStub);
    reader.on(STHeadersReader.EVENTS.RESET, resetHandlerStub);

    await reader.read();

    expect(resetHandlerStub).to.be.calledOnce();
    expect(blockHandlerStub).to.be.callCount(rpcClientMock.blocks.length);
  });

  it('should emit "staleBlock" if previous block hash not equal to current block previous hash', async function it() {
    const previousBlockIndex = 1;
    const previousBlock = rpcClientMock.blocks[previousBlockIndex];
    const readerState = new STHeadersReaderState([previousBlock]);
    const reader = new STHeadersReader(stateTransitionHeaderIterator, readerState);

    const currentBlock = { previousblockhash: 'wrong', height: 3 };
    rpcClientMock.getBlock.callThrough()
      .withArgs(rpcClientMock.blocks[previousBlockIndex + 1].hash)
      .onCall(0)
      .returns(Promise.resolve({ result: currentBlock }));

    const blockHandlerStub = this.sinon.stub();
    const staleBlockHandlerStub = this.sinon.stub();

    reader.on(STHeadersReader.EVENTS.BLOCK, blockHandlerStub);
    reader.on(STHeadersReader.EVENTS.STALE_BLOCK, staleBlockHandlerStub);

    await reader.read();

    expect(staleBlockHandlerStub).to.be.calledOnce();
    expect(staleBlockHandlerStub).to.be.calledWith(previousBlock);

    expect(blockHandlerStub).has.callCount(rpcClientMock.blocks.length);
    rpcClientMock.blocks.forEach((block, i) => {
      expect(blockHandlerStub.getCall(i)).to.be.calledWith(block);
    });
  });

  it('should emit "staleBlock" for synced blocks if current block height is lower than last synced block', async function it() {
    const { blocks } = rpcClientMock;

    const readerState = new STHeadersReaderState(blocks);
    const reader = new STHeadersReader(stateTransitionHeaderIterator, readerState);

    blockIterator.setBlockHeight(2);

    const blockHandlerStub = this.sinon.stub();
    const staleBlockHandlerStub = this.sinon.stub();

    reader.on(STHeadersReader.EVENTS.BLOCK, blockHandlerStub);
    reader.on(STHeadersReader.EVENTS.STALE_BLOCK, staleBlockHandlerStub);

    await reader.read();

    expect(staleBlockHandlerStub).to.be.calledThrice();
    expect(staleBlockHandlerStub.firstCall).to.be.calledWith(blocks[3]);
    expect(staleBlockHandlerStub.secondCall).to.be.calledWith(blocks[2]);
    expect(staleBlockHandlerStub.thirdCall).to.be.calledWith(blocks[1]);

    expect(blockHandlerStub).to.be.calledThrice();
    expect(blockHandlerStub.firstCall).to.be.calledWith(blocks[1]);
    expect(blockHandlerStub.secondCall).to.be.calledWith(blocks[2]);
    expect(blockHandlerStub.thirdCall).to.be.calledWith(blocks[3]);
  });
});
