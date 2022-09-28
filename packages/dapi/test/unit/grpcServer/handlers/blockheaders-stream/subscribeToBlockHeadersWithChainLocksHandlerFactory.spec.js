const { ChainLock } = require('@dashevo/dashcore-lib');

const {
  server: {
    error: {
      NotFoundGrpcError,
      InvalidArgumentGrpcError,
    },
    stream: {
      AcknowledgingWritable,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    BlockHeadersWithChainLocksRequest,
    BlockHeadersWithChainLocksResponse,
    BlockHeaders,
  },
} = require('@dashevo/dapi-grpc');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');
const subscribeToBlockHeadersWithChainLocksHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/blockheaders-stream/subscribeToBlockHeadersWithChainLocksHandlerFactory',
);
const ChainDataProvider = require('../../../../../lib/chainDataProvider/ChainDataProvider');

let coreAPIMock;
let zmqClientMock;

describe('subscribeToBlockHeadersWithChainLocksHandlerFactory', () => {
  let call;
  let subscribeToBlockHeadersWithChainLocksHandler;
  let getHistoricalBlockHeadersIteratorMock;
  let subscribeToNewBlockHeadersMock;
  let chainDataProvider;

  const blockHash = Buffer.from('00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc2c', 'hex');

  beforeEach(function beforeEach() {
    coreAPIMock = {
      getBlock: this.sinon.stub(),
      getBlockStats: this.sinon.stub(),
      getBlockHeaders: this.sinon.stub(),
      getBestBlockHeight: this.sinon.stub(),
      getBlockHash: this.sinon.stub(),
      getBestChainLock: this.sinon.stub(),
    };
    subscribeToNewBlockHeadersMock = this.sinon.stub();

    async function* asyncGenerator() {
      yield [{ toBuffer: () => Buffer.from('fake', 'utf-8') }];
    }

    getHistoricalBlockHeadersIteratorMock = () => asyncGenerator();
    zmqClientMock = { on: this.sinon.stub(), topics: { hashblock: 'fake' } };

    chainDataProvider = new ChainDataProvider(coreAPIMock, zmqClientMock);

    // eslint-disable-next-line operator-linebreak
    subscribeToBlockHeadersWithChainLocksHandler =
      subscribeToBlockHeadersWithChainLocksHandlerFactory(
        getHistoricalBlockHeadersIteratorMock,
        coreAPIMock,
        chainDataProvider,
        zmqClientMock,
        subscribeToNewBlockHeadersMock,
      );
  });

  it('should subscribe to newBlockHeaders', async function it() {
    this.sinon.stub(AcknowledgingWritable.prototype, 'write');

    let request = new BlockHeadersWithChainLocksRequest();

    request.setFromBlockHash(blockHash);
    request.setCount(0);

    request = BlockHeadersWithChainLocksRequest.deserializeBinary(request.serializeBinary());

    call = new GrpcCallMock(this.sinon, request);

    coreAPIMock.getBestChainLock.resolves({
      height: 1,
      signature: Buffer.from('fakeSig'),
      blockHash,
    });
    coreAPIMock.getBlockStats.resolves({ height: 1 });

    await subscribeToBlockHeadersWithChainLocksHandler(call);
    expect(subscribeToNewBlockHeadersMock).to.have.been.called();
    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(blockHash
      .toString('hex'), ['height']);
  });

  it('should subscribe from block hash', async function it() {
    const writableStub = this.sinon.stub(AcknowledgingWritable.prototype, 'write');
    let request = new BlockHeadersWithChainLocksRequest();

    request.setFromBlockHash(blockHash);
    request.setCount(0);

    request = BlockHeadersWithChainLocksRequest.deserializeBinary(request.serializeBinary());

    call = new GrpcCallMock(this.sinon, request);

    // monkey-patching
    chainDataProvider.chainLock = new ChainLock({
      height: 1,
      signature: Buffer.from('fakesig', 'hex'),
      blockHash: Buffer.from('fakeHash', 'hex'),
    });

    coreAPIMock.getBlockStats.resolves({ height: 1 });

    await subscribeToBlockHeadersWithChainLocksHandler(call);

    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(
      blockHash.toString('hex'),
      ['height'],
    );

    const clSigResponse = new BlockHeadersWithChainLocksResponse();
    clSigResponse.setChainLock(new ChainLock({
      height: 1,
      signature: Buffer.from('fakesig', 'hex'),
      blockHash: Buffer.from('fakeHash', 'hex'),
    }).toBuffer());

    expect(writableStub.getCall(0).args).to.deep.equal(
      [clSigResponse],
    );

    const blockHeadersProto = new BlockHeaders();
    blockHeadersProto.setHeadersList(
      [Buffer.from('fake', 'utf-8')],
    );
    const iteratorResponse = new BlockHeadersWithChainLocksResponse();
    iteratorResponse.setBlockHeaders(blockHeadersProto);

    expect(writableStub.getCall(1).args).to.deep.equal(
      [iteratorResponse],
    );
  });

  it('should subscribe from block height', async function it() {
    this.sinon.stub(AcknowledgingWritable.prototype, 'write');

    const blockHeight = 1;
    const count = 5;

    let request = new BlockHeadersWithChainLocksRequest();
    request.setFromBlockHeight(blockHeight);
    request.setCount(count);

    request = BlockHeadersWithChainLocksRequest.deserializeBinary(request.serializeBinary());

    call = new GrpcCallMock(this.sinon, request);

    coreAPIMock.getBestChainLock.resolves({
      height: 1,
      signature: Buffer.from('fakeSig'),
      blockHash: Buffer.from('fakeHash'),
    });

    coreAPIMock.getBlockStats.resolves({ height: 1 });

    await subscribeToBlockHeadersWithChainLocksHandler(call);

    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(
      blockHeight,
      ['height'],
    );

    expect(subscribeToNewBlockHeadersMock).to.not.have.been.called();
  });

  it('should handle getBlockStats RPC method errors', async function it() {
    let request = new BlockHeadersWithChainLocksRequest();

    request.setFromBlockHash(blockHash);
    request.setCount(0);

    request = BlockHeadersWithChainLocksRequest.deserializeBinary(request.serializeBinary());

    call = new GrpcCallMock(this.sinon, request);

    try {
      coreAPIMock.getBlockStats.throws({ code: -5 });

      await subscribeToBlockHeadersWithChainLocksHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.instanceOf(NotFoundGrpcError);
      expect(e.message).to.be.equal(`Block ${blockHash.toString('hex')} not found`);
    }

    try {
      coreAPIMock.getBlockStats.throws({ code: -8 });

      await subscribeToBlockHeadersWithChainLocksHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.instanceOf(NotFoundGrpcError);
      expect(e.message).to.be.equal(`Block ${blockHash.toString('hex')} not found`);
    }

    try {
      request.setCount(10);

      coreAPIMock.getBlockStats.resolves({ height: 10 });

      coreAPIMock.getBestBlockHeight.resolves(11);

      await subscribeToBlockHeadersWithChainLocksHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
    }
  });
});
