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

const ChainInfo = require('../../../../lib/chainInfo/ChainInfo');

const RootTreeMock = require('../../../../lib/test/mock/RootTreeMock');
const packageJson = require('../../../../package');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');

const CreditsDistributionPool = require('../../../../lib/creditsDistributionPool/CreditsDistributionPool');

describe('infoHandlerFactory', () => {
  let protocolVersion;
  let lastBlockHeight;
  let lastBlockAppHash;
  let infoHandler;
  let rootTreeMock;
  let updateSimplifiedMasternodeListMock;
  let lastCoreChainLockedHeight;
  let loggerMock;
  let chainInfo;
  let chainInfoRepositoryMock;
  let containerMock;
  let previousBlockExecutionStoreTransactionsRepositoryMock;
  let blockExecutionStoreTransactionsMock;
  let creditsDistributionPoolRepositoryMock;
  let creditsDistributionPool;

  beforeEach(function beforeEach() {
    lastBlockHeight = Long.fromInt(0);
    lastBlockAppHash = Buffer.alloc(0);
    protocolVersion = Long.fromInt(0);
    lastCoreChainLockedHeight = 0;

    creditsDistributionPool = new CreditsDistributionPool();

    creditsDistributionPoolRepositoryMock = {
      fetch: this.sinon.stub().resolves(creditsDistributionPool),
    };

    chainInfo = new ChainInfo(
      lastBlockHeight,
      lastCoreChainLockedHeight,
    );

    chainInfoRepositoryMock = {
      store: this.sinon.stub(),
      fetch: this.sinon.stub().resolves(chainInfo),
    };

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

    infoHandler = infoHandlerFactory(
      chainInfo,
      chainInfoRepositoryMock,
      creditsDistributionPool,
      creditsDistributionPoolRepositoryMock,
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

    expect(chainInfoRepositoryMock.fetch).to.be.calledOnceWithExactly();
    expect(creditsDistributionPoolRepositoryMock.fetch).to.be.calledOnceWithExactly();

    expect(previousBlockExecutionStoreTransactionsRepositoryMock.fetch).to.not.be.called();
    expect(containerMock.has).to.not.be.called();
  });

  it('should update SML to latest core chain locked height and return stored info', async () => {
    lastBlockHeight = Long.fromInt(1);
    lastCoreChainLockedHeight = 2;

    chainInfo.setLastBlockHeight(lastBlockHeight);
    chainInfo.setLastCoreChainLockedHeight(lastCoreChainLockedHeight);

    const response = await infoHandler();

    expect(response).to.be.an.instanceOf(ResponseInfo);

    expect(response).to.deep.include({
      version: packageJson.version,
      appVersion: protocolVersion,
      lastBlockHeight,
      lastBlockAppHash,
    });

    expect(updateSimplifiedMasternodeListMock).to.be.calledOnceWithExactly(
      lastCoreChainLockedHeight,
      {
        logger: loggerMock,
      },
    );

    expect(chainInfoRepositoryMock.fetch).to.be.calledOnceWithExactly();
    expect(creditsDistributionPoolRepositoryMock.fetch).to.be.calledOnceWithExactly();

    expect(previousBlockExecutionStoreTransactionsRepositoryMock.fetch).to.be.calledWithExactly();
    expect(containerMock.has).to.be.calledOnceWithExactly('previousBlockExecutionStoreTransactions');
  });

  it('should throw NoPreviousBlockExecutionStoreTransactionsFoundError if previous BlockExecutionStoreTransactions is not present', async () => {
    lastBlockHeight = Long.fromInt(1);
    lastCoreChainLockedHeight = 2;

    chainInfo.setLastBlockHeight(lastBlockHeight);
    chainInfo.setLastCoreChainLockedHeight(lastCoreChainLockedHeight);

    previousBlockExecutionStoreTransactionsRepositoryMock.fetch.resolves(null);

    try {
      await infoHandler();

      expect.fail('should throw NoPreviousBlockExecutionStoreTransactionsFoundError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(NoPreviousBlockExecutionStoreTransactionsFoundError);
    }
  });
});
