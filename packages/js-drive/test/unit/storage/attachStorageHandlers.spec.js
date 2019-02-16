const BlockchainReaderMediatorMock = require('../../../lib/test/mock/BlockchainReaderMediatorMock');

const ReaderMediator = require('../../../lib/blockchain/reader/BlockchainReaderMediator');
const RpcClientMock = require('../../../lib/test/mock/RpcClientMock');

const attachStorageHandlers = require('../../../lib/storage/attachStorageHandlers');

describe('attachStorageHandlers', () => {
  let rpcClientMock;
  let readerMediatorMock;
  let stPacketRepositoryMock;

  beforeEach(function beforeEach() {
    rpcClientMock = new RpcClientMock(this.sinon);

    readerMediatorMock = new BlockchainReaderMediatorMock(this.sinon);

    stPacketRepositoryMock = {
      download: this.sinon.stub(),
      delete: this.sinon.stub(),
      deleteAll: this.sinon.stub(),
    };

    attachStorageHandlers(
      readerMediatorMock,
      stPacketRepositoryMock,
    );
  });

  it('should pin ST packet when new state transition appears', async () => {
    const [stateTransition] = rpcClientMock.transactions;
    const [block] = rpcClientMock.blocks;

    await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.STATE_TRANSITION, {
      stateTransition,
      block,
    });

    const packetCid = stateTransition.getPacketCID();

    expect(stPacketRepositoryMock.download).to.be.calledOnce();
    expect(stPacketRepositoryMock.download).to.be.calledWith(packetCid);
  });

  it('should unpin ST packets in case of reorg', async () => {
    const [stateTransition] = rpcClientMock.transactions;
    const [block] = rpcClientMock.blocks;

    await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.STATE_TRANSITION_ORPHANED, {
      stateTransition,
      block,
    });

    const packetCid = stateTransition.getPacketCID();

    expect(stPacketRepositoryMock.delete).to.be.calledOnce();
    expect(stPacketRepositoryMock.delete).to.be.calledWith(packetCid);
  });

  it('should unpin all packets on reset event', async () => {
    await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.RESET);

    expect(stPacketRepositoryMock.deleteAll).to.be.calledOnce();
  });
});
