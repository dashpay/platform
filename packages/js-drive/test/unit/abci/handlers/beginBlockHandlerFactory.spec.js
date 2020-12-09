const Long = require('long');

const {
  abci: {
    ResponseBeginBlock,
  },
} = require('abci/types');

const beginBlockHandlerFactory = require('../../../../lib/abci/handlers/beginBlockHandlerFactory');

const ChainInfo = require('../../../../lib/chainInfo/ChainInfo');
const BlockExecutionDBTransactionsMock = require('../../../../lib/test/mock/BlockExecutionStoreTransactionsMock');
const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');

describe('beginBlockHandlerFactory', () => {
  let protocolVersion;
  let beginBlockHandler;
  let request;
  let chainInfo;
  let blockHeight;
  let blockExecutionDBTransactionsMock;
  let blockExecutionContextMock;
  let header;

  beforeEach(function beforeEach() {
    chainInfo = new ChainInfo();

    protocolVersion = Long.fromInt(0);

    blockExecutionDBTransactionsMock = new BlockExecutionDBTransactionsMock(this.sinon);

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    const loggerMock = {
      debug: this.sinon.stub(),
      info: this.sinon.stub(),
    };

    beginBlockHandler = beginBlockHandlerFactory(
      chainInfo,
      blockExecutionDBTransactionsMock,
      blockExecutionContextMock,
      protocolVersion,
      loggerMock,
    );

    blockHeight = 2;

    header = {
      version: {
        App: protocolVersion,
      },
      height: blockHeight,
      time: {
        seconds: Math.ceil(new Date().getTime() / 1000),
      },
    };

    request = {
      header,
    };
  });

  it('should update height, start transactions return ResponseBeginBlock', async () => {
    const response = await beginBlockHandler(request);

    expect(response).to.be.an.instanceOf(ResponseBeginBlock);

    expect(chainInfo.getLastBlockHeight()).to.equal(blockHeight);
    expect(blockExecutionDBTransactionsMock.start).to.be.calledOnce();
    expect(blockExecutionContextMock.reset).to.be.calledOnce();
    expect(blockExecutionContextMock.setHeader).to.be.calledOnceWithExactly(header);
  });

  it('should reject not supported protocol version', async () => {
    request.header.version.App = Long.fromInt(42);

    try {
      await beginBlockHandler(request);

      expect.fail('Expected exception to be thrown');
    } catch (err) {
      expect(err).to.be.an('Error');
      expect(err.message).to.equal('Block protocol version 42 not supported. Expected to be less or equal to 0.');
      expect(err.name).to.equal('NotSupportedProtocolVersionError');
    }
  });
});
