const Long = require('long');

const {
  tendermint: {
    abci: {
      ResponseBeginBlock,
    },
  },
} = require('@dashevo/abci/types');

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
  let coreHeight;
  let blockExecutionDBTransactionsMock;
  let blockExecutionContextMock;
  let header;
  let updateSimplifiedMasternodeListMock;
  let waitForChainlockedHeightMock;

  beforeEach(function beforeEach() {
    chainInfo = new ChainInfo();

    protocolVersion = Long.fromInt(0);

    blockExecutionDBTransactionsMock = new BlockExecutionDBTransactionsMock(this.sinon);

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    const loggerMock = {
      debug: this.sinon.stub(),
      info: this.sinon.stub(),
    };

    updateSimplifiedMasternodeListMock = this.sinon.stub();
    waitForChainlockedHeightMock = this.sinon.stub();

    beginBlockHandler = beginBlockHandlerFactory(
      chainInfo,
      blockExecutionDBTransactionsMock,
      blockExecutionContextMock,
      protocolVersion,
      updateSimplifiedMasternodeListMock,
      waitForChainlockedHeightMock,
      loggerMock,
    );

    blockHeight = 2;
    blockHeight = 1;

    header = {
      version: {
        app: protocolVersion,
      },
      height: blockHeight,
      time: {
        seconds: Math.ceil(new Date().getTime() / 1000),
      },
      coreHeight,
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
    expect(updateSimplifiedMasternodeListMock).to.be.calledOnceWithExactly(coreHeight);
    expect(waitForChainlockedHeightMock).to.be.calledOnceWithExactly(coreHeight);
  });

  it('should reject not supported protocol version', async () => {
    request.header.version.app = Long.fromInt(42);

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
