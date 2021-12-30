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

let coreAPIMock;
let zmqClientMock;

describe('subscribeToBlockHeadersWithChainLocksHandlerFactory', () => {
  let call;
  let subscribeToBlockHeadersWithChainLocksHandler;
  let getHistoricalBlockHeadersIteratorMock;
  let subscribeToNewBlockHeadersMock;

  beforeEach(function () {
    coreAPIMock = {
      getBlock: this.sinon.stub(),
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

    // eslint-disable-next-line operator-linebreak
    subscribeToBlockHeadersWithChainLocksHandler =
      subscribeToBlockHeadersWithChainLocksHandlerFactory(
        getHistoricalBlockHeadersIteratorMock,
        coreAPIMock,
        zmqClientMock,
        subscribeToNewBlockHeadersMock,
      );
  });

  it('should subscribe to newBlockHeaders', async function () {
    this.sinon.stub(AcknowledgingWritable.prototype, 'write');
    const request = new BlockHeadersWithChainLocksRequest();

    call = new GrpcCallMock(this.sinon, request);

    call.request.setFromBlockHash('fakehash');
    call.request.setCount(0);

    coreAPIMock.getBestChainLock.resolves({
      height: 1,
      signature: Buffer.from('fakeSig'),
      blockHash: Buffer.from('fakeHash'),
    });
    coreAPIMock.getBlock.resolves({ height: 1 });

    await subscribeToBlockHeadersWithChainLocksHandler(call);
    expect(subscribeToNewBlockHeadersMock).to.have.been.called();
  });

  it('should subscribe from block hash', async function () {
    const writableStub = this.sinon.stub(AcknowledgingWritable.prototype, 'write');
    const request = new BlockHeadersWithChainLocksRequest();

    call = new GrpcCallMock(this.sinon, request);

    call.request.setFromBlockHash('someBlockHash');
    call.request.setCount(0);

    coreAPIMock.getBestChainLock.resolves({
      height: 1,
      signature: Buffer.from('fakesig', 'hex'),
      blockHash: Buffer.from('fakeHash', 'hex'),
    });

    try {
      coreAPIMock.getBlock.resolves({ height: -1 });
      await subscribeToBlockHeadersWithChainLocksHandler(call);
    } catch (e) {
      expect(e).to.be.instanceOf(NotFoundGrpcError);
      expect(e.message).to.be('fromBlockHash is not found');
    }

    try {
      coreAPIMock.getBlock.resolves({ height: 1, confirmations: -1 });
      await subscribeToBlockHeadersWithChainLocksHandler(call);
    } catch (e) {
      expect(e).to.be.instanceOf(NotFoundGrpcError);
      expect(e.message.includes('is not part of the best block chain')).to.be.true();
    }

    try {
      coreAPIMock.getBlock.resolves({ height: 10 });
      coreAPIMock.getBestBlockHeight.resolves(11);
      await subscribeToBlockHeadersWithChainLocksHandler(call);
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
    }

    await subscribeToBlockHeadersWithChainLocksHandler(call);

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

  it('should subscribe from block height', async function () {
    this.sinon.stub(AcknowledgingWritable.prototype, 'write');
    const request = new BlockHeadersWithChainLocksRequest();

    call = new GrpcCallMock(this.sinon, request);

    call.request.setFromBlockHeight(1);
    call.request.setCount(5);

    coreAPIMock.getBestChainLock.resolves({
      height: 1,
      signature: Buffer.from('fakeSig'),
      blockHash: Buffer.from('fakeHash'),
    });
    coreAPIMock.getBlock.resolves({ height: 1 });

    await subscribeToBlockHeadersWithChainLocksHandler(call);
    expect(subscribeToNewBlockHeadersMock).to.not.have.been.called();
  });
});
