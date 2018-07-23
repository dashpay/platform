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

    attachStorageHandlers(stHeadersReaderMock, ipfsAPIMock, unpinAllIpfsPackets);
  });

  it('should pin ST packets when new header appears', async () => {
    const [header] = rpcClientMock.transitionHeaders;

    const pinPromise = Promise.resolve();
    ipfsAPIMock.pin.add.returns(pinPromise);

    await stHeadersReaderMock.emitSerial(STHeadersReaderEvents.HEADER, { header });

    expect(ipfsAPIMock.pin.add).to.be.calledOnce();
    expect(ipfsAPIMock.pin.add).to.be.calledWith(header.getPacketCID(), { recursive: true });

    expect(rejectAfterMock).to.be.calledOnce();

    const calledWithArgs = rejectAfterMock.firstCall.args;

    expect(calledWithArgs[0]).to.be.equal(pinPromise);
    expect(calledWithArgs[1].name).to.be.equal('InvalidPacketCidError');
    expect(calledWithArgs[2]).to.be.equal(attachStorageHandlers.PIN_REJECTION_TIMEOUT);
  });

  it('should unpin ST packets in case of reorg', async () => {
    const [block] = rpcClientMock.blocks;

    await stHeadersReaderMock.emitSerial(STHeadersReaderEvents.STALE_BLOCK, block);

    expect(ipfsAPIMock.pin.rm).has.callCount(block.ts.length);

    rpcClientMock.transitionHeaders.slice(0, block.ts.length).forEach((header) => {
      expect(ipfsAPIMock.pin.rm).to.be.calledWith(header.getPacketCID(), { recursive: true });
    });
  });

  it('should call unpinAllIpfsPackets on stHeadersReader reset event', async () => {
    await stHeadersReaderMock.emit(STHeadersReaderEvents.RESET);
    expect(unpinAllIpfsPackets).to.be.calledOnce();
  });
});
