const Emitter = require('emittery');

const RpcClientMock = require('../../../lib/test/mock/RpcClientMock');
const attachPinSTPacketHandler = require('../../../lib/storage/attachPinSTPacketHandler');

describe('attachPinSTPacketHandler', () => {
  let rpcClientMock;
  let ipfsAPIMock;
  let stHeadersReaderMock;

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
    class STHeadersReader extends Emitter {
      constructor() {
        super();

        this.stHeaderIterator = {
          rpcClient: rpcClientMock,
        };
      }
    }

    stHeadersReaderMock = new STHeadersReader();

    attachPinSTPacketHandler(stHeadersReaderMock, ipfsAPIMock);
  });

  it('should pin ST packets when new header appears', async () => {
    const [header] = rpcClientMock.transitionHeaders;

    await stHeadersReaderMock.emitSerial('header', header);

    expect(ipfsAPIMock.pin.add).to.be.calledOnce();
    expect(ipfsAPIMock.pin.add).to.be.calledWith(header.getPacketCID(), { recursive: true });
  });

  it('should unpin ST packets in case of reorg', async () => {
    const [block] = rpcClientMock.blocks;

    await stHeadersReaderMock.emitSerial('wrongSequence', block);

    expect(ipfsAPIMock.pin.rm).has.callCount(block.ts.length);

    rpcClientMock.transitionHeaders.slice(0, block.ts.length).forEach((header) => {
      expect(ipfsAPIMock.pin.rm).to.be.calledWith(header.getPacketCID(), { recursive: true });
    });
  });
});
