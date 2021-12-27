const { expect, use } = require('chai');
// eslint-disable-next-line no-underscore-dangle
const _sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
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
    ChainLockSignatureMessages,
  },
} = require('@dashevo/dapi-grpc');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');
const subscribeToBlockHeadersWithChainLocksHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/blockheaders-stream/subscribeToBlockHeadersWithChainLocksHandlerFactory',
);

let sinon;
let coreAPIMock;
let zmqClientMock;

use(sinonChai);
use(chaiAsPromised);
use(dirtyChai);

describe('subscribeToTransactionsWithProofsHandlerFactory', () => {
  afterEach(() => {
    sinon.restore();
  });

  let call;
  let subscribeToBlockHeadersWithChainLocksHandler;
  let getHistoricalBlockHeadersIteratorMock;
  let subscribeToNewBlockHeadersMock;

  beforeEach(() => {
    if (!sinon) {
      sinon = _sinon.createSandbox();
    } else {
      sinon.restore();
    }

    coreAPIMock = {
      getBlock: sinon.stub(),
      getBestBlockHeight: sinon.stub(),
      getBlockHash: sinon.stub(),
      getBestChainLock: sinon.stub(),
    };
    subscribeToNewBlockHeadersMock = sinon.stub();

    async function* asyncGenerator() {
      yield [{ toBuffer: () => Buffer.from('fake', 'utf-8') }];
    }

    getHistoricalBlockHeadersIteratorMock = () => asyncGenerator();
    zmqClientMock = { on: sinon.stub(), topics: { hashblock: 'fake' } };

    // eslint-disable-next-line operator-linebreak
    subscribeToBlockHeadersWithChainLocksHandler =
      subscribeToBlockHeadersWithChainLocksHandlerFactory(
        getHistoricalBlockHeadersIteratorMock,
        coreAPIMock,
        zmqClientMock,
        subscribeToNewBlockHeadersMock,
      );
  });

  it('should subscribe to newBlockHeaders', async () => {
    sinon.stub(AcknowledgingWritable.prototype, 'write');
    const request = new BlockHeadersWithChainLocksRequest();

    call = new GrpcCallMock(sinon, request);

    call.request.setFromBlockHash('fakehash');
    call.request.setCount(0);

    coreAPIMock.getBestChainLock.resolves({ signature: 'fakesig' });
    coreAPIMock.getBlock.resolves({ height: 1 });

    await subscribeToBlockHeadersWithChainLocksHandler(call);
    expect(subscribeToNewBlockHeadersMock).to.have.been.called();
  });

  it('should subscribe from block hash', async () => {
    const writableStub = sinon.stub(AcknowledgingWritable.prototype, 'write');
    const request = new BlockHeadersWithChainLocksRequest();

    call = new GrpcCallMock(sinon, request);

    call.request.setFromBlockHash('someBlockHash');
    call.request.setCount(0);

    coreAPIMock.getBestChainLock.resolves({ signature: 'fakesig' });

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

    const clSigMessages = new ChainLockSignatureMessages();
    clSigMessages.setMessagesList([Buffer.from('fakesig', 'hex')]);
    const clSigResponse = new BlockHeadersWithChainLocksResponse();
    clSigResponse.setChainLockSignatureMessages(clSigMessages);

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

  it('should subscribe from block height', async () => {
    sinon.stub(AcknowledgingWritable.prototype, 'write');
    const request = new BlockHeadersWithChainLocksRequest();

    call = new GrpcCallMock(sinon, request);

    call.request.setFromBlockHeight(1);
    call.request.setCount(5);

    coreAPIMock.getBestChainLock.resolves({ signature: 'fakesig' });
    coreAPIMock.getBlock.resolves({ height: 1 });

    await subscribeToBlockHeadersWithChainLocksHandler(call);
    expect(subscribeToNewBlockHeadersMock).to.not.have.been.called();
  });
});
