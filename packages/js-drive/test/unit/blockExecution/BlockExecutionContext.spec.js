const {
  tendermint: {
    abci: {
      CommitInfo,
    },
    version: {
      Consensus,
    },
  },
} = require('@dashevo/abci/types');

const Long = require('long');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const BlockExecutionContext = require('../../../lib/blockExecution/BlockExecutionContext');
const getBlockExecutionContextObjectFixture = require('../../../lib/test/fixtures/getBlockExecutionContextObjectFixture');

describe('BlockExecutionContext', () => {
  let blockExecutionContext;
  let dataContract;
  let lastCommitInfo;
  let logger;
  let plainObject;
  let height;
  let coreChainLockedHeight;
  let version;
  let epochInfo;
  let timeMs;
  let prepareProposalResult;

  beforeEach(() => {
    blockExecutionContext = new BlockExecutionContext();
    dataContract = getDataContractFixture();
    delete dataContract.entropy;

    plainObject = getBlockExecutionContextObjectFixture(dataContract);

    lastCommitInfo = CommitInfo.fromObject(plainObject.lastCommitInfo);

    logger = plainObject.contextLogger;
    height = Long.fromNumber(plainObject.height);
    coreChainLockedHeight = plainObject.coreChainLockedHeight;
    version = Consensus.fromObject(plainObject.version);
    epochInfo = plainObject.epochInfo;
    timeMs = plainObject.timeMs;
    prepareProposalResult = plainObject.prepareProposalResult;
  });

  describe('#addDataContract', () => {
    it('should add a Data Contract', async () => {
      expect(blockExecutionContext.getDataContracts()).to.have.lengthOf(0);

      blockExecutionContext.addDataContract(dataContract);
      const contracts = blockExecutionContext.getDataContracts();

      expect(contracts).to.have.lengthOf(1);
      expect(contracts[0]).to.deep.equal(dataContract);
    });
  });

  describe('#hasDataContract', () => {
    it('should respond with false if data contract with specified ID is not present', async () => {
      const result = blockExecutionContext.hasDataContract(dataContract.getId());

      expect(result).to.be.false();
    });

    it('should respond with true if data contract with specified ID is present', async () => {
      blockExecutionContext.addDataContract(dataContract);

      const result = blockExecutionContext.hasDataContract(dataContract.getId());

      expect(result).to.be.true();
    });
  });

  describe('#getDataContracts', () => {
    it('should get data contracts', async () => {
      blockExecutionContext.addDataContract(dataContract);
      blockExecutionContext.addDataContract(dataContract);

      const contracts = blockExecutionContext.getDataContracts();

      expect(contracts).to.have.lengthOf(2);
      expect(contracts[0]).to.deep.equal(dataContract);
      expect(contracts[1]).to.deep.equal(dataContract);
    });
  });

  describe('#reset', () => {
    it('should reset state', () => {
      blockExecutionContext.addDataContract(dataContract);

      expect(blockExecutionContext.getDataContracts()).to.have.lengthOf(1);

      blockExecutionContext.reset();

      expect(blockExecutionContext.getDataContracts()).to.have.lengthOf(0);

      expect(blockExecutionContext.getHeight()).to.be.null();
      expect(blockExecutionContext.getCoreChainLockedHeight()).to.be.null();
      expect(blockExecutionContext.getVersion()).to.be.null();
      expect(blockExecutionContext.getTimeMs()).to.be.null();
      expect(blockExecutionContext.getLastCommitInfo()).to.be.null();
      expect(blockExecutionContext.getWithdrawalTransactionsMap()).to.deep.equal({});
    });
  });

  describe('#setCoreChainLockedHeight', () => {
    it('should set coreChainLockedHeight', async () => {
      const result = blockExecutionContext.setCoreChainLockedHeight(coreChainLockedHeight);

      expect(result).to.equal(blockExecutionContext);

      expect(blockExecutionContext.coreChainLockedHeight).to.deep.equal(coreChainLockedHeight);
    });
  });

  describe('#getCoreChainLockedHeight', () => {
    it('should get coreChainLockedHeight', async () => {
      blockExecutionContext.coreChainLockedHeight = coreChainLockedHeight;

      expect(blockExecutionContext.getCoreChainLockedHeight()).to.deep.equal(coreChainLockedHeight);
    });
  });

  describe('#setHeight', () => {
    it('should set height', async () => {
      const result = blockExecutionContext.setHeight(height);

      expect(result).to.equal(blockExecutionContext);

      expect(blockExecutionContext.height).to.deep.equal(height);
    });
  });

  describe('#getHeight', () => {
    it('should get height', async () => {
      blockExecutionContext.height = height;

      expect(blockExecutionContext.getHeight()).to.deep.equal(height);
    });
  });

  describe('#setVersion', () => {
    it('should set version', async () => {
      const result = blockExecutionContext.setVersion(version);

      expect(result).to.equal(blockExecutionContext);

      expect(blockExecutionContext.version).to.deep.equal(version);
    });
  });

  describe('#getVersion', () => {
    it('should get version', async () => {
      blockExecutionContext.version = version;

      expect(blockExecutionContext.getVersion()).to.deep.equal(version);
    });
  });

  describe('#setLastCommitInfo', () => {
    it('should set lastCommitInfo', async () => {
      const result = blockExecutionContext.setLastCommitInfo(lastCommitInfo);

      expect(result).to.equal(blockExecutionContext);

      expect(blockExecutionContext.lastCommitInfo).to.deep.equal(lastCommitInfo);
    });
  });

  describe('#getLastCommitInfo', () => {
    it('should get lastCommitInfo', async () => {
      blockExecutionContext.lastCommitInfo = lastCommitInfo;

      expect(blockExecutionContext.getLastCommitInfo()).to.deep.equal(lastCommitInfo);
    });
  });

  describe('#setWithdrawalTransactionsMap', () => {
    it('should set withdrawalTransactionsMap', async () => {
      const result = blockExecutionContext.setWithdrawalTransactionsMap(
        plainObject.withdrawalTransactionsMap,
      );

      expect(result).to.equal(blockExecutionContext);

      expect(blockExecutionContext.withdrawalTransactionsMap).to.deep.equal(
        plainObject.withdrawalTransactionsMap,
      );
    });
  });

  describe('#getWithdrawalTransactionsMap', () => {
    it('should get withdrawalTransactionsMap', async () => {
      blockExecutionContext.withdrawalTransactionsMap = plainObject.withdrawalTransactionsMap;

      expect(blockExecutionContext.getWithdrawalTransactionsMap()).to.deep.equal(
        plainObject.withdrawalTransactionsMap,
      );
    });
  });

  describe('#setRound', () => {
    it('should set round', async () => {
      const result = blockExecutionContext.setRound(
        plainObject.round,
      );

      expect(result).to.equal(blockExecutionContext);

      expect(blockExecutionContext.round).to.deep.equal(
        plainObject.round,
      );
    });
  });

  describe('#getRound', () => {
    it('should get round', async () => {
      blockExecutionContext.round = plainObject.round;

      expect(blockExecutionContext.getRound()).to.deep.equal(
        plainObject.round,
      );
    });
  });

  describe('#setPrepareProposalResult', () => {
    it('should set PrepareProposal result', async () => {
      const result = blockExecutionContext.setPrepareProposalResult(
        plainObject.prepareProposalResult,
      );

      expect(result).to.equal(blockExecutionContext);

      expect(blockExecutionContext.prepareProposalResult).to.deep.equal(
        plainObject.prepareProposalResult,
      );
    });
  });

  describe('#getPrepareProposalResult', () => {
    it('should get PrepareProposal result', async () => {
      blockExecutionContext.prepareProposalResult = plainObject.prepareProposalResult;

      expect(blockExecutionContext.getPrepareProposalResult()).to.deep.equal(
        plainObject.prepareProposalResult,
      );
    });
  });

  describe('#setTimeMs', () => {
    it('should set time', async () => {
      blockExecutionContext.setTimeMs(timeMs);

      expect(blockExecutionContext.timeMs).to.deep.equal(timeMs);
    });
  });

  describe('#getTimeMs', () => {
    it('should get time', async () => {
      blockExecutionContext.timeMs = timeMs;

      expect(blockExecutionContext.getTimeMs()).to.deep.equal(timeMs);
    });
  });

  describe('#setEpochInfo', () => {
    it('should set epoch info');
  });

  describe('#getEpochInfo', () => {
    it('should return epoch info');
  });

  describe('#populate', () => {
    it('should populate instance from another instance', () => {
      const anotherBlockExecutionContext = new BlockExecutionContext();

      anotherBlockExecutionContext.dataContracts = [dataContract];
      anotherBlockExecutionContext.lastCommitInfo = lastCommitInfo;
      anotherBlockExecutionContext.height = height;
      anotherBlockExecutionContext.version = version;
      anotherBlockExecutionContext.coreChainLockedHeight = coreChainLockedHeight;
      anotherBlockExecutionContext.contextLogger = logger;
      anotherBlockExecutionContext.withdrawalTransactionsMap = plainObject
        .withdrawalTransactionsMap;
      anotherBlockExecutionContext.epochInfo = epochInfo;
      anotherBlockExecutionContext.timeMs = timeMs;

      blockExecutionContext.populate(anotherBlockExecutionContext);

      expect(blockExecutionContext.dataContracts).to.equal(
        anotherBlockExecutionContext.dataContracts,
      );
      expect(blockExecutionContext.lastCommitInfo).to.equal(
        anotherBlockExecutionContext.lastCommitInfo,
      );
      expect(blockExecutionContext.height).to.equal(
        anotherBlockExecutionContext.height,
      );
      expect(blockExecutionContext.version).to.equal(
        anotherBlockExecutionContext.version,
      );
      expect(blockExecutionContext.coreChainLockedHeight).to.equal(
        anotherBlockExecutionContext.coreChainLockedHeight,
      );
      expect(blockExecutionContext.contextLogger).to.equal(
        anotherBlockExecutionContext.contextLogger,
      );
      expect(blockExecutionContext.withdrawalTransactionsMap).to.equal(
        anotherBlockExecutionContext.withdrawalTransactionsMap,
      );
      expect(blockExecutionContext.epochInfo).to.equal(
        anotherBlockExecutionContext.epochInfo,
      );
      expect(blockExecutionContext.timeMs).to.equal(
        anotherBlockExecutionContext.timeMs,
      );
    });
  });

  describe('#toObject', () => {
    it('should return a plain object', () => {
      blockExecutionContext.dataContracts = [dataContract];
      blockExecutionContext.lastCommitInfo = lastCommitInfo;
      blockExecutionContext.height = height;
      blockExecutionContext.coreChainLockedHeight = coreChainLockedHeight;
      blockExecutionContext.version = version;
      blockExecutionContext.contextLogger = logger;
      blockExecutionContext.epochInfo = epochInfo;
      blockExecutionContext.timeMs = timeMs;
      blockExecutionContext.withdrawalTransactionsMap = plainObject.withdrawalTransactionsMap;
      blockExecutionContext.round = plainObject.round;
      blockExecutionContext.prepareProposalResult = plainObject.prepareProposalResult;

      expect(blockExecutionContext.toObject()).to.deep.equal(plainObject);
    });

    it('should skipContextLogger if the option passed', () => {
      blockExecutionContext.dataContracts = [dataContract];
      blockExecutionContext.lastCommitInfo = lastCommitInfo;
      blockExecutionContext.height = height;
      blockExecutionContext.coreChainLockedHeight = coreChainLockedHeight;
      blockExecutionContext.version = version;
      blockExecutionContext.contextLogger = logger;
      blockExecutionContext.withdrawalTransactionsMap = plainObject.withdrawalTransactionsMap;
      blockExecutionContext.round = plainObject.round;
      blockExecutionContext.epochInfo = epochInfo;
      blockExecutionContext.timeMs = timeMs;
      blockExecutionContext.prepareProposalResult = prepareProposalResult;

      const result = blockExecutionContext.toObject({ skipContextLogger: true });

      delete plainObject.contextLogger;

      expect(result).to.deep.equal(plainObject);
    });

    it('should skipPrepareProposalResult if the option passed', () => {
      blockExecutionContext.dataContracts = [dataContract];
      blockExecutionContext.lastCommitInfo = lastCommitInfo;
      blockExecutionContext.height = height;
      blockExecutionContext.coreChainLockedHeight = coreChainLockedHeight;
      blockExecutionContext.version = version;
      blockExecutionContext.contextLogger = logger;
      blockExecutionContext.withdrawalTransactionsMap = plainObject.withdrawalTransactionsMap;
      blockExecutionContext.round = plainObject.round;
      blockExecutionContext.epochInfo = epochInfo;
      blockExecutionContext.timeMs = timeMs;

      const result = blockExecutionContext.toObject({ skipPrepareProposalResult: true });

      delete plainObject.prepareProposalResult;

      expect(result).to.deep.equal(plainObject);
    });
  });

  describe('#fromObject', () => {
    it('should populate instance from a plain object', () => {
      blockExecutionContext.fromObject(plainObject);

      if (blockExecutionContext.dataContracts[0].$defs === undefined) {
        blockExecutionContext.dataContracts[0].$defs = {};
      }

      expect(blockExecutionContext.dataContracts).to.have.deep.members(
        [dataContract],
      );
      expect(blockExecutionContext.lastCommitInfo).to.deep.equal(lastCommitInfo);
      expect(blockExecutionContext.height).to.deep.equal(height);
      expect(blockExecutionContext.version).to.deep.equal(version);
      expect(blockExecutionContext.coreChainLockedHeight).to.deep.equal(coreChainLockedHeight);
      expect(blockExecutionContext.contextLogger).to.equal(logger);
      expect(blockExecutionContext.withdrawalTransactionsMap).to.deep.equal(
        plainObject.withdrawalTransactionsMap,
      );
      expect(blockExecutionContext.timeMs).to.equal(timeMs);
    });
  });
});
