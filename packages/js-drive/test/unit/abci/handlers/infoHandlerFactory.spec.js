const Long = require('long');

const {
  tendermint: {
    abci: {
      ResponseInfo,
    },
  },
} = require('@dashevo/abci/types');

const NoPreviousBlockExecutionStoreTransactionsFoundError = require('../../../../lib/abci/handlers/errors/NoPreviousBlockExecutionStoreTransactionsFoundError');

const infoHandlerFactory = require('../../../../lib/abci/handlers/infoHandlerFactory');

const RootTreeMock = require('../../../../lib/test/mock/RootTreeMock');
const packageJson = require('../../../../package.json');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');

const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');

describe('infoHandlerFactory', () => {
  let protocolVersion;
  let lastBlockHeight;
  let lastBlockAppHash;
  let infoHandler;
  let rootTreeMock;
  let updateSimplifiedMasternodeListMock;
  let lastCoreChainLockedHeight;
  let loggerMock;
  let containerMock;
  let previousBlockExecutionStoreTransactionsRepositoryMock;
  let blockExecutionStoreTransactionsMock;
  let blockExecutionContextMock;

  beforeEach(function beforeEach() {
    lastBlockHeight = Long.fromInt(0);
    lastBlockAppHash = Buffer.alloc(0);
    protocolVersion = Long.fromInt(1);
    lastCoreChainLockedHeight = 0;

    rootTreeMock = new RootTreeMock(this.sinon);
    rootTreeMock.getRootHash.returns(lastBlockAppHash);

    updateSimplifiedMasternodeListMock = this.sinon.stub();

    loggerMock = new LoggerMock(this.sinon);

    containerMock = {
      register: this.sinon.stub(),
      has: this.sinon.stub().withArgs('previousBlockExecutionStoreTransactions').returns(false),
    };

    blockExecutionStoreTransactionsMock = {};

    previousBlockExecutionStoreTransactionsRepositoryMock = {
      fetch: this.sinon.stub().resolves(blockExecutionStoreTransactionsMock),
    };

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    infoHandler = infoHandlerFactory(
      blockExecutionContextMock,
      protocolVersion,
      rootTreeMock,
      updateSimplifiedMasternodeListMock,
      loggerMock,
      previousBlockExecutionStoreTransactionsRepositoryMock,
      containerMock,
    );
  });

  it('should return empty info', async () => {
    const response = await infoHandler();

    expect(response).to.be.an.instanceOf(ResponseInfo);

    expect(response).to.deep.include({
      version: packageJson.version,
      appVersion: protocolVersion,
      lastBlockHeight,
      lastBlockAppHash,
    });

    expect(updateSimplifiedMasternodeListMock).to.not.be.called();

    expect(previousBlockExecutionStoreTransactionsRepositoryMock.fetch).to.not.be.called();
    expect(containerMock.has).to.not.be.called();
  });

  it('should update SML to latest core chain locked height and return stored info', async () => {
    lastBlockHeight = Long.fromInt(1);
    lastCoreChainLockedHeight = 2;

    blockExecutionContextMock.getHeader.returns({
      height: lastBlockHeight,
      coreChainLockedHeight: lastCoreChainLockedHeight,
    });

    const response = await infoHandler();

    expect(response).to.be.an.instanceOf(ResponseInfo);

    expect(ResponseInfo.toObject(response)).to.deep.equal({
      version: packageJson.version,
      appVersion: protocolVersion,
      lastBlockHeight,
      lastBlockAppHash,
      lastCoreChainLockedHeight,
    });

    expect(updateSimplifiedMasternodeListMock).to.be.calledOnceWithExactly(
      lastCoreChainLockedHeight,
      {
        logger: loggerMock,
      },
    );

    expect(previousBlockExecutionStoreTransactionsRepositoryMock.fetch).to.be.calledWithExactly();
    expect(containerMock.has).to.be.calledOnceWithExactly('previousBlockExecutionStoreTransactions');
  });

  it('should throw NoPreviousBlockExecutionStoreTransactionsFoundError if previous BlockExecutionStoreTransactions is not present', async () => {
    lastBlockHeight = Long.fromInt(1);
    lastCoreChainLockedHeight = 2;

    blockExecutionContextMock.getHeader.returns({
      height: lastBlockHeight,
      coreChainLockedHeight: lastCoreChainLockedHeight,
    });

    previousBlockExecutionStoreTransactionsRepositoryMock.fetch.resolves(null);

    try {
      await infoHandler();

      expect.fail('should throw NoPreviousBlockExecutionStoreTransactionsFoundError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(NoPreviousBlockExecutionStoreTransactionsFoundError);
    }
  });
});
