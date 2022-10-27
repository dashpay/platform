const Long = require('long');
const {
  tendermint: {
    version: {
      Consensus,
    },
  },
  google: {
    protobuf: {
      Timestamp,
    },
  },
} = require('@dashevo/abci/types');
const { hash } = require('@dashevo/dpp/lib/util/hash');

const beginBlockFactory = require('../../../../../lib/abci/handlers/proposal/beginBlockFactory');

const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');
const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');
const NotSupportedNetworkProtocolVersionError = require('../../../../../lib/abci/handlers/errors/NotSupportedNetworkProtocolVersionError');
const NetworkProtocolVersionIsNotSetError = require('../../../../../lib/abci/handlers/errors/NetworkProtocolVersionIsNotSetError');
const GroveDBStoreMock = require('../../../../../lib/test/mock/GroveDBStoreMock');
const ProposalBlockExecutionContextCollection = require('../../../../../lib/blockExecution/ProposalBlockExecutionContextCollection');

describe('beginBlockFactory', () => {
  let protocolVersion;
  let beginBlock;
  let request;
  let blockHeight;
  let coreChainLockedHeight;
  let blockExecutionContextMock;
  let updateSimplifiedMasternodeListMock;
  let waitForChainLockedHeightMock;
  let loggerMock;
  let lastCommitInfo;
  let dppMock;
  let transactionalDppMock;
  let synchronizeMasternodeIdentitiesMock;
  let groveDBStoreMock;
  let version;
  let time;
  let rsAbciMock;
  let proposerProTxHash;
  let round;
  let executionTimerMock;
  let latestBlockExecutionContextMock;
  let proposalBlockExecutionContextCollection;

  beforeEach(function beforeEach() {
    round = 0;
    protocolVersion = Long.fromInt(1);

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    latestBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    loggerMock = new LoggerMock(this.sinon);

    dppMock = {
      setProtocolVersion: this.sinon.stub(),
    };
    transactionalDppMock = {
      setProtocolVersion: this.sinon.stub(),
    };

    executionTimerMock = {
      clearTimer: this.sinon.stub(),
      startTimer: this.sinon.stub(),
      stopTimer: this.sinon.stub(),
    };

    updateSimplifiedMasternodeListMock = this.sinon.stub().resolves(false);
    waitForChainLockedHeightMock = this.sinon.stub();
    synchronizeMasternodeIdentitiesMock = this.sinon.stub().resolves({
      createdEntities: [],
      updatedEntities: [],
      removedEntities: [],
      fromHeight: 1,
      toHeight: 42,
    });

    groveDBStoreMock = new GroveDBStoreMock(this.sinon);

    rsAbciMock = {
      blockBegin: this.sinon.stub(),
    };

    rsAbciMock.blockBegin.resolves({});

    proposalBlockExecutionContextCollection = new ProposalBlockExecutionContextCollection();

    beginBlock = beginBlockFactory(
      groveDBStoreMock,
      latestBlockExecutionContextMock,
      proposalBlockExecutionContextCollection,
      protocolVersion,
      dppMock,
      transactionalDppMock,
      updateSimplifiedMasternodeListMock,
      waitForChainLockedHeightMock,
      synchronizeMasternodeIdentitiesMock,
      rsAbciMock,
      executionTimerMock,
    );

    blockHeight = new Long(1);

    lastCommitInfo = {};

    version = Consensus.fromObject({
      app: protocolVersion,
    });

    time = new Timestamp({
      seconds: Long.fromNumber(Math.ceil(new Date().getTime() / 1000)),
    });

    proposerProTxHash = Buffer.alloc(32, 1);

    request = {
      height: blockHeight,
      lastCommitInfo,
      coreChainLockedHeight,
      version,
      time,
      proposerProTxHash,
      round,
    };
  });

  it('should reset previous block state and prepare everything for for a next one', async () => {
    await beginBlock(request, loggerMock);

    // Wait for chain locked core block height
    expect(waitForChainLockedHeightMock).to.be.calledOnceWithExactly(coreChainLockedHeight);

    // Set current protocol version
    expect(dppMock.setProtocolVersion).to.have.been.calledOnceWithExactly(
      protocolVersion.toNumber(),
    );
    expect(transactionalDppMock.setProtocolVersion).to.have.been.calledOnceWithExactly(
      protocolVersion.toNumber(),
    );

    // Start new transaction
    expect(groveDBStoreMock.startTransaction).to.be.calledOnceWithExactly();

    // Update SML
    expect(updateSimplifiedMasternodeListMock).to.be.calledOnceWithExactly(
      coreChainLockedHeight, { logger: loggerMock },
    );

    expect(synchronizeMasternodeIdentitiesMock).to.not.been.called();

    expect(executionTimerMock.clearTimer).to.be.calledTwice();
    expect(executionTimerMock.clearTimer.getCall(1)).to.be.calledWithExactly('roundExecution');
    expect(executionTimerMock.clearTimer.getCall(0)).to.be.calledWithExactly('blockExecution');

    expect(executionTimerMock.startTimer).to.be.calledTwice();
    expect(executionTimerMock.startTimer.getCall(1)).to.be.calledWithExactly('roundExecution');
    expect(executionTimerMock.startTimer.getCall(0)).to.be.calledWithExactly('blockExecution');

    const executionContext = proposalBlockExecutionContextCollection.get(round);

    expect(executionContext.consensusLogger).to.equal(loggerMock);
    expect(executionContext.height).to.equal(blockHeight);
    expect(executionContext.version).to.equal(version);
    expect(executionContext.time).to.equal(time);
    expect(executionContext.coreChainLockedHeight).to.equal(coreChainLockedHeight);
    expect(executionContext.lastCommitInfo).to.equal(lastCommitInfo);
  });

  it('should synchronize masternode identities if SML is updated', async () => {
    updateSimplifiedMasternodeListMock.resolves(true);

    await beginBlock(request, loggerMock);

    expect(synchronizeMasternodeIdentitiesMock).to.have.been.calledOnceWithExactly(
      coreChainLockedHeight,
    );
  });

  it('should throw NotSupportedNetworkProtocolVersionError if protocol version is not supported', async () => {
    request.version.app = Long.fromInt(42);

    try {
      await beginBlock(request, loggerMock);

      expect.fail('should throw NotSupportedNetworkProtocolVersionError');
    } catch (e) {
      expect(e).to.be.instanceOf(NotSupportedNetworkProtocolVersionError);
      expect(e.getNetworkProtocolVersion()).to.equal(request.version.app);
      expect(e.getLatestProtocolVersion()).to.equal(protocolVersion);
    }
  });

  it('should throw an NetworkProtocolVersionIsNotSetError if network protocol version is not set', async () => {
    request.version.app = Long.fromInt(0);

    try {
      await beginBlock(request, loggerMock);

      expect.fail('should throw NetworkProtocolVersionIsNotSetError');
    } catch (err) {
      expect(err).to.be.an.instanceOf(NetworkProtocolVersionIsNotSetError);
    }
  });

  it('should set withdrawal transactions map if present', async () => {
    const [txOneBytes, txTwoBytes] = [
      Buffer.alloc(32, 0),
      Buffer.alloc(32, 1),
    ];

    rsAbciMock.blockBegin.resolves({
      unsignedWithdrawalTransactions: [txOneBytes, txTwoBytes],
    });

    await beginBlock(request, loggerMock);

    const executionContext = proposalBlockExecutionContextCollection.get(round);

    expect(executionContext.withdrawalTransactionsMap).to.deep.equal({
      [hash(txOneBytes).toString('hex')]: txOneBytes,
      [hash(txTwoBytes).toString('hex')]: txTwoBytes,
    });
  });
});
