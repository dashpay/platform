const ReaderMediator = require('../../../../lib/blockchain/reader/BlockchainReaderMediator');

const readBlockchainFactory = require('../../../../lib/blockchain/reader/readBlockchainFactory');

const BlockchainReaderMediatorMock = require('../../../../lib/test/mock/BlockchainReaderMediatorMock');

const RpcClientMock = require('../../../../lib/test/mock/RpcClientMock');

const getBlockFixtures = require('../../../../lib/test/fixtures/getBlocksFixture');

describe('readBlockchainFactory', () => {
  let readerMediatorMock;
  let rpcClientMock;
  let readerMock;
  let readBlockchain;
  let initialBlockHeight;
  let currentBlockCount;
  let blocks;

  beforeEach(function beforeEach() {
    readerMediatorMock = new BlockchainReaderMediatorMock(this.sinon);

    readerMock = {
      read: this.sinon.stub(),
    };

    rpcClientMock = new RpcClientMock(this.sinon);

    blocks = getBlockFixtures();

    initialBlockHeight = 2;
    currentBlockCount = rpcClientMock.blocks.length;

    readBlockchain = readBlockchainFactory(
      readerMock,
      readerMediatorMock,
      rpcClientMock,
    );
  });

  it('should emit the out of bounds event if the initial block height is less than'
    + ' the number of blocks on the blockchain', async () => {
    initialBlockHeight = 6;

    readerMediatorMock.getInitialBlockHeight.returns(initialBlockHeight);

    await readBlockchain();

    expect(readerMediatorMock.reset).to.have.not.been.called();

    expect(readerMediatorMock.emitSerial).to.have.been.calledOnceWith(
      ReaderMediator.EVENTS.OUT_OF_BOUNDS,
      {
        initialBlockHeight,
        currentBlockCount,
      },
    );

    expect(readerMock.read).to.have.not.been.called();
  });

  it('should reset state and emit the out of bounds event if the initial block height'
    + ' is less than the current one and Drive has already synced something', async () => {
    initialBlockHeight = 6;

    readerMediatorMock.getInitialBlockHeight.returns(initialBlockHeight);

    readerMediatorMock.getState().getLastBlock.returns(blocks[0]);

    await readBlockchain();

    expect(readerMediatorMock.reset).to.have.been.calledOnce();

    expect(readerMediatorMock.emitSerial).to.have.been.calledOnceWith(
      ReaderMediator.EVENTS.OUT_OF_BOUNDS,
      {
        initialBlockHeight,
        currentBlockCount,
      },
    );

    expect(readerMock.read).to.have.not.been.called();
  });

  it('should emit the fully synced event if the last synced block and the last block'
    + ' from the chain are the same', async () => {
    initialBlockHeight = currentBlockCount;

    readerMediatorMock.getInitialBlockHeight.returns(initialBlockHeight);

    readerMediatorMock.getState().getLastBlock.returns(blocks[3]);

    await readBlockchain();

    expect(readerMediatorMock.reset).to.have.not.been.called();

    expect(readerMediatorMock.emitSerial).to.have.been.calledOnceWith(
      ReaderMediator.EVENTS.FULLY_SYNCED,
      currentBlockCount,
    );

    expect(readerMock.read).to.have.not.been.called();
  });

  it('should read from the next block after the last synced block if the blockchain height is the same but'
    + ' block hashes are different', async () => {
    initialBlockHeight = currentBlockCount;

    const [, , , lastSyncedBlock] = blocks;

    lastSyncedBlock.hash = 'wrong';

    const nextBlockHeight = lastSyncedBlock.height + 1;

    const readBlockCount = 10;

    readerMock.read.returns(readBlockCount);

    readerMediatorMock.getInitialBlockHeight.returns(initialBlockHeight);

    readerMediatorMock.getState().getLastBlock.returns(lastSyncedBlock);

    await readBlockchain();

    expect(readerMediatorMock.reset).to.have.not.been.called();

    expect(readerMock.read).to.have.been.calledOnceWith(nextBlockHeight);

    expect(readerMediatorMock.emitSerial).to.have.been.calledTwice();

    expect(readerMediatorMock.emitSerial).to.have.been.calledWith(
      ReaderMediator.EVENTS.BEGIN,
      nextBlockHeight,
    );

    expect(readerMediatorMock.emitSerial).to.have.been.calledWith(
      ReaderMediator.EVENTS.END,
      readBlockCount,
    );
  });

  it('should reset the state if there is no previous block to rely onto', async () => {
    const [lastSyncedBlock] = blocks;
    const firstSycedBlock = 4;

    lastSyncedBlock.height = 7;

    readerMock.read.returns(currentBlockCount);

    readerMediatorMock.getInitialBlockHeight.returns(initialBlockHeight);
    readerMediatorMock.getState().getFirstBlockHeight.returns(firstSycedBlock);
    readerMediatorMock.getState().getLastBlock.returns(lastSyncedBlock);

    await readBlockchain();

    expect(readerMediatorMock.reset).to.have.been.calledOnce();

    expect(readerMock.read).to.have.been.calledOnceWith(initialBlockHeight);

    expect(readerMediatorMock.emitSerial).to.have.callCount(3);

    expect(readerMediatorMock.emitSerial).to.have.been.calledWith(
      ReaderMediator.EVENTS.BEGIN,
      initialBlockHeight,
    );

    expect(readerMediatorMock.emitSerial).to.have.been.calledWith(
      ReaderMediator.EVENTS.BLOCK_SEQUENCE_VALIDATION_IMPOSSIBLE,
      {
        height: currentBlockCount,
        firstSyncedBlockHeight: firstSycedBlock,
      },
    );

    expect(readerMediatorMock.emitSerial).to.have.been.calledWith(
      ReaderMediator.EVENTS.END,
      currentBlockCount,
    );
  });

  it('should continue from the current blockchain height if it is less than'
    + ' the last synced block height but higher than the first synced block', async () => {
    const readBlockCount = 10;
    const [lastSyncedBlock] = blocks;

    lastSyncedBlock.height = 7;

    readerMock.read.returns(readBlockCount);

    readerMediatorMock.getInitialBlockHeight.returns(initialBlockHeight);
    readerMediatorMock.getState().getFirstBlockHeight.returns(2);
    readerMediatorMock.getState().getLastBlock.returns(lastSyncedBlock);

    await readBlockchain();

    expect(readerMediatorMock.reset).to.have.not.been.called();

    expect(readerMock.read).to.have.been.calledOnceWith(currentBlockCount);

    expect(readerMediatorMock.emitSerial).to.have.been.calledTwice();

    expect(readerMediatorMock.emitSerial).to.have.been.calledWith(
      ReaderMediator.EVENTS.BEGIN,
      currentBlockCount,
    );
    expect(readerMediatorMock.emitSerial).to.have.been.calledWith(
      ReaderMediator.EVENTS.END,
      readBlockCount,
    );
  });

  it('should read from the next block after the last synced block', async () => {
    const [, , lastSyncedBlock] = blocks;

    const nextBlockHeight = lastSyncedBlock.height + 1;
    const readBlockCount = 10;

    readerMock.read.returns(readBlockCount);

    readerMediatorMock.getInitialBlockHeight.returns(initialBlockHeight);

    readerMediatorMock.getState().getLastBlock.returns(lastSyncedBlock);

    await readBlockchain();

    expect(readerMediatorMock.reset).to.have.not.been.called();

    expect(readerMock.read).to.have.been.calledOnceWith(nextBlockHeight);

    expect(readerMediatorMock.emitSerial).to.have.been.calledTwice();

    expect(readerMediatorMock.emitSerial).to.have.been.calledWith(
      ReaderMediator.EVENTS.BEGIN,
      nextBlockHeight,
    );

    expect(readerMediatorMock.emitSerial).to.have.been.calledWith(
      ReaderMediator.EVENTS.END,
      readBlockCount,
    );
  });

  it('should read from the initial block height if it is less than the blockchain height'
    + ' and there are no synced blocks', async () => {
    const readBlockCount = 10;

    readerMock.read.returns(readBlockCount);

    readerMediatorMock.getInitialBlockHeight.returns(initialBlockHeight);

    await readBlockchain();

    expect(readerMediatorMock.reset).to.have.not.been.called();

    expect(readerMediatorMock.emitSerial).to.have.been.calledTwice();

    expect(readerMock.read).to.have.been.calledOnceWith(initialBlockHeight);

    expect(readerMediatorMock.emitSerial).to.have.been.calledWith(
      ReaderMediator.EVENTS.BEGIN,
      initialBlockHeight,
    );

    expect(readerMediatorMock.emitSerial).to.have.been.calledWith(
      ReaderMediator.EVENTS.END,
      readBlockCount,
    );
  });
});
