const Long = require('long');

const {
  abci: {
    ResponseBeginBlock,
  },
} = require('abci/types');

const beginBlockHandlerFactory = require('../../../../lib/abci/handlers/beginBlockHandlerFactory');

const BlockchainState = require('../../../../lib/blockchainState/BlockchainState');
const BlockExecutionDBTransactionsMock = require('../../../../lib/test/mock/BlockExecutionDBTransactionsMock');
const BlockExecutionStateMock = require('../../../../lib/test/mock/BlockExecutionStateMock');

describe('beginBlockHandlerFactory', () => {
  let protocolVersion;
  let beginBlockHandler;
  let request;
  let blockchainState;
  let blockHeight;
  let blockExecutionDBTransactionsMock;
  let blockExecutionStateMock;
  let header;

  beforeEach(function beforeEach() {
    blockchainState = new BlockchainState();

    protocolVersion = Long.fromInt(0);

    blockExecutionDBTransactionsMock = new BlockExecutionDBTransactionsMock(this.sinon);

    blockExecutionStateMock = new BlockExecutionStateMock(this.sinon);

    const loggerMock = {
      debug: this.sinon.stub(),
      info: this.sinon.stub(),
    };

    beginBlockHandler = beginBlockHandlerFactory(
      blockchainState,
      blockExecutionDBTransactionsMock,
      blockExecutionStateMock,
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

    expect(blockchainState.getLastBlockHeight()).to.equal(blockHeight);
    expect(blockExecutionDBTransactionsMock.start).to.be.calledOnce();
    expect(blockExecutionStateMock.reset).to.be.calledOnce();
    expect(blockExecutionStateMock.setHeader).to.be.calledOnceWithExactly(header);
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
