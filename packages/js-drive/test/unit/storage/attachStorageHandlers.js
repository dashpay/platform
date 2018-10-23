const Emitter = require('emittery');
const proxyquire = require('proxyquire');

const { EVENTS: STHeadersReaderEvents } = require('../../../lib/blockchain/reader/STHeadersReader');
const RpcClientMock = require('../../../lib/test/mock/RpcClientMock');

describe('attachStorageHandlers', () => {
  let rpcClientMock;
  let ipfsAPIMock;
  let stHeadersReaderMock;
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

    // Mock STHeadersReader
    class STHeadersReaderMock extends Emitter {
      constructor() {
        super();

        this.stHeaderIterator = {
          rpcClient: rpcClientMock,
        };
      }
    }

    stHeadersReaderMock = new STHeadersReaderMock();
    unpinAllIpfsPackets = this.sinon.stub();

    rejectAfterMock = this.sinon.stub();
    attachStorageHandlers = proxyquire('../../../lib/storage/attachStorageHandlers', {
      '../util/rejectAfter': rejectAfterMock,
    });

    ipfsTimeout = 1;

    attachStorageHandlers(
      stHeadersReaderMock,
      ipfsAPIMock,
      unpinAllIpfsPackets,
      ipfsTimeout,
    );
  });

  it('should pin ST packets when new header appears', async () => {
    const [header] = rpcClientMock.transitionHeaders;

    const pinPromise = Promise.resolve();
    ipfsAPIMock.pin.add.returns(pinPromise);

    await stHeadersReaderMock.emitSerial(STHeadersReaderEvents.HEADER, { header });

    const packetPath = header.getPacketCID().toBaseEncodedString();
    expect(ipfsAPIMock.pin.add).to.be.calledOnce();
    expect(ipfsAPIMock.pin.add).to.be.calledWith(packetPath, { recursive: true });

    expect(rejectAfterMock).to.be.calledOnce();

    const calledWithArgs = rejectAfterMock.firstCall.args;

    expect(calledWithArgs[0]).to.be.equal(pinPromise);
    expect(calledWithArgs[1].name).to.be.equal('PinPacketTimeoutError');
    expect(calledWithArgs[2]).to.be.equal(ipfsTimeout);
  });

  it('should unpin ST packets in case of reorg', async () => {
    const [block] = rpcClientMock.blocks;

    await stHeadersReaderMock.emitSerial(STHeadersReaderEvents.STALE_BLOCK, block);

    expect(ipfsAPIMock.pin.rm).has.callCount(block.tx.length);

    rpcClientMock.transitionHeaders.slice(0, block.tx.length).forEach((header) => {
      const packetPath = header.getPacketCID().toBaseEncodedString();
      expect(ipfsAPIMock.pin.rm).to.be.calledWith(packetPath, { recursive: true });
    });
  });

  it('should call unpinAllIpfsPackets on stHeadersReader reset event', async () => {
    await stHeadersReaderMock.emit(STHeadersReaderEvents.RESET);
    expect(unpinAllIpfsPackets).to.be.calledOnce();
  });
});
