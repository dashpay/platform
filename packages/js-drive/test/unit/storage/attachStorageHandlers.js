const proxyquire = require('proxyquire');

const BlockchainReaderMediatorMock = require('../../../lib/test/mock/BlockchainReaderMediatorMock');

const ReaderMediator = require('../../../lib/blockchain/reader/BlockchainReaderMediator');
const RpcClientMock = require('../../../lib/test/mock/RpcClientMock');

describe('attachStorageHandlers', () => {
  let rpcClientMock;
  let ipfsAPIMock;
  let readerMediatorMock;
  let rejectAfterMock;
  let attachStorageHandlers;
  let unpinAllIpfsPackets;
  let ipfsTimeout = 1;

  beforeEach(function beforeEach() {
    rpcClientMock = new RpcClientMock(this.sinon);

    // Mock IPFS API
    const sinonSandbox = this.sinon;
    class IpfsAPI {
      constructor() {
        this.pin = {
          add: sinonSandbox.stub(),
          rm: sinonSandbox.stub(),
        };
      }
    }

    ipfsAPIMock = new IpfsAPI();

    readerMediatorMock = new BlockchainReaderMediatorMock(this.sinon);
    unpinAllIpfsPackets = this.sinon.stub();

    rejectAfterMock = this.sinon.stub();
    attachStorageHandlers = proxyquire('../../../lib/storage/attachStorageHandlers', {
      '../util/rejectAfter': rejectAfterMock,
    });

    ipfsTimeout = 1;

    attachStorageHandlers(
      readerMediatorMock,
      ipfsAPIMock,
      rpcClientMock,
      unpinAllIpfsPackets,
      ipfsTimeout,
    );
  });

  it('should pin ST packet when new state transition appears', async () => {
    const [stateTransition] = rpcClientMock.transitionHeaders;
    const [block] = rpcClientMock.blocks;

    const pinPromise = Promise.resolve();
    ipfsAPIMock.pin.add.returns(pinPromise);

    await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.STATE_TRANSITION, {
      stateTransition,
      block,
    });

    const packetPath = stateTransition.getPacketCID().toBaseEncodedString();

    expect(ipfsAPIMock.pin.add).to.be.calledOnce();
    expect(ipfsAPIMock.pin.add).to.be.calledWith(packetPath, { recursive: true });

    expect(rejectAfterMock).to.be.calledOnce();

    const calledWithArgs = rejectAfterMock.firstCall.args;

    expect(calledWithArgs[0]).to.be.equal(pinPromise);
    expect(calledWithArgs[1].name).to.be.equal('PinPacketTimeoutError');
    expect(calledWithArgs[2]).to.be.equal(ipfsTimeout);
  });

  it('should unpin ST packets in case of reorg', async () => {
    const [stateTransition] = rpcClientMock.transitionHeaders;
    const [block] = rpcClientMock.blocks;

    await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.STATE_TRANSITION_STALE, {
      stateTransition,
      block,
    });

    const packetPath = stateTransition.getPacketCID().toBaseEncodedString();

    expect(ipfsAPIMock.pin.rm).to.be.calledOnce();
    expect(ipfsAPIMock.pin.rm).to.be.calledWith(packetPath, { recursive: true });
  });

  it('should call unpinAllIpfsPackets on stHeadersReader reset event', async () => {
    await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.RESET);

    expect(unpinAllIpfsPackets).to.be.calledOnce();
  });
});
