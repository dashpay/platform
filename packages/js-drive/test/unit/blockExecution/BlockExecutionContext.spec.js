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
  let cumulativeProcessingFee;
  let cumulativeStorageFee;
  let plainObject;
  let height;
  let previousHeight;
  let coreChainLockedHeight;
  let previousCoreChainLockedHeight;
  let version;
  let time;
  let previousTime;

  beforeEach(() => {
    blockExecutionContext = new BlockExecutionContext();
    dataContract = getDataContractFixture();
    delete dataContract.entropy;

    plainObject = getBlockExecutionContextObjectFixture(dataContract);

    lastCommitInfo = CommitInfo.fromObject(plainObject.lastCommitInfo);

    logger = plainObject.consensusLogger;
    cumulativeProcessingFee = plainObject.cumulativeProcessingFee;
    cumulativeStorageFee = plainObject.cumulativeStorageFee;
    height = Long.fromNumber(plainObject.height);
    previousHeight = Long.fromNumber(plainObject.previousHeight);
    coreChainLockedHeight = plainObject.coreChainLockedHeight;
    version = Consensus.fromObject(plainObject.version);
    time = plainObject.time;
    time.seconds = Long.fromNumber(time.seconds);
    previousTime = plainObject.previousBlockTime;
    previousTime.seconds = Long.fromNumber(previousTime.seconds);
    previousCoreChainLockedHeight = plainObject.previousCoreChainLockedHeight;
    plainObject.time = plainObject.time.toJSON();
    plainObject.time.seconds = Number(plainObject.time.seconds);
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

  describe('#getCumulativeProcessingFee', () => {
    it('should get cumulative fees', async () => {
      let result = blockExecutionContext.getCumulativeProcessingFee();

      expect(result).to.equal(0);

      blockExecutionContext.cumulativeProcessingFee = cumulativeProcessingFee;

      result = blockExecutionContext.getCumulativeProcessingFee();

      expect(result).to.equal(cumulativeProcessingFee);
    });
  });

  describe('#getCumulativeStorageFee', () => {
    it('should get cumulative fees', async () => {
      let result = blockExecutionContext.getCumulativeStorageFee();

      expect(result).to.equal(0);

      blockExecutionContext.cumulativeStorageFee = cumulativeStorageFee;

      result = blockExecutionContext.getCumulativeStorageFee();

      expect(result).to.equal(cumulativeStorageFee);
    });
  });

  describe('#incrementCumulativeProcessingFee', () => {
    it('should increment cumulative fees', async () => {
      let result = blockExecutionContext.getCumulativeProcessingFee();

      expect(result).to.equal(0);

      blockExecutionContext.incrementCumulativeProcessingFee(15);

      result = blockExecutionContext.getCumulativeProcessingFee();

      expect(result).to.equal(15);
    });
  });

  describe('#incrementCumulativeStorageFee', () => {
    it('should increment cumulative fees', async () => {
      let result = blockExecutionContext.getCumulativeStorageFee();

      expect(result).to.equal(0);

      blockExecutionContext.incrementCumulativeStorageFee(15);

      result = blockExecutionContext.getCumulativeStorageFee();

      expect(result).to.equal(15);
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
      expect(blockExecutionContext.getTime()).to.be.null();
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

  describe('#setTime', () => {
    it('should set time', async () => {
      const result = blockExecutionContext.setTime(time);

      expect(result).to.equal(blockExecutionContext);

      expect(blockExecutionContext.time).to.deep.equal(time);
    });
  });

  describe('#getTime', () => {
    it('should get time', async () => {
      blockExecutionContext.time = time;

      expect(blockExecutionContext.getTime()).to.deep.equal(time);
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

  describe('#setPreviousHeight', () => {
    it('should set previousHeight', async () => {
      const result = blockExecutionContext.setPreviousHeight(previousHeight);

      expect(result).to.deep.equal(blockExecutionContext);

      expect(blockExecutionContext.previousHeight).to.deep.equal(previousHeight);
    });
  });

  describe('#getPreviousHeight', () => {
    it('should get previousHeight', async () => {
      blockExecutionContext.previousHeight = previousHeight;

      expect(blockExecutionContext.getPreviousHeight()).to.deep.equal(previousHeight);
    });
  });

  describe('#setPreviousTime', () => {
    it('should set previousTime', async () => {
      const result = blockExecutionContext.setPreviousTime(previousTime);

      expect(result).to.deep.equal(blockExecutionContext);

      expect(blockExecutionContext.previousBlockTime).to.deep.equal(previousTime);
    });
  });

  describe('#getPreviousTime', () => {
    it('should get previousTime', async () => {
      blockExecutionContext.previousBlockTime = previousTime;

      expect(blockExecutionContext.getPreviousTime()).to.deep.equal(previousTime);
    });
  });

  describe('#setPreviousCoreChainLockedHeight', () => {
    it('should set previousCoreChainLockedHeight', () => {
      const result = blockExecutionContext.setPreviousCoreChainLockedHeight(
        previousCoreChainLockedHeight,
      );

      expect(result).to.deep.equal(blockExecutionContext);

      expect(blockExecutionContext.previousCoreChainLockedHeight).to.deep.equal(
        previousCoreChainLockedHeight,
      );
    });
  });

  describe('#getPreviousCoreChainLockedHeight', () => {
    it('should get previousCoreChainLockedHeight', () => {
      blockExecutionContext.previousCoreChainLockedHeight = previousCoreChainLockedHeight;

      expect(blockExecutionContext.getPreviousCoreChainLockedHeight()).to.deep.equal(
        previousCoreChainLockedHeight,
      );
    });
  });

  describe('#init', () => {
    it('should init state', () => {
      blockExecutionContext.setPreviousCoreChainLockedHeight(
        previousCoreChainLockedHeight,
      );
      blockExecutionContext.setPreviousTime(previousTime);
      blockExecutionContext.setPreviousHeight(previousHeight);

      blockExecutionContext.init();

      expect(blockExecutionContext.getPreviousCoreChainLockedHeight()).to.be.null();
      expect(blockExecutionContext.getPreviousTime()).to.be.null();
      expect(blockExecutionContext.getPreviousHeight()).to.be.null();
    });
  });

  describe('#populate', () => {
    it('should populate instance from another instance', () => {
      const anotherBlockExecutionContext = new BlockExecutionContext();

      anotherBlockExecutionContext.dataContracts = [dataContract];
      anotherBlockExecutionContext.lastCommitInfo = lastCommitInfo;
      anotherBlockExecutionContext.cumulativeProcessingFee = cumulativeProcessingFee;
      anotherBlockExecutionContext.cumulativeStorageFee = cumulativeStorageFee;
      anotherBlockExecutionContext.height = height;
      anotherBlockExecutionContext.time = time;
      anotherBlockExecutionContext.version = version;
      anotherBlockExecutionContext.coreChainLockedHeight = coreChainLockedHeight;
      anotherBlockExecutionContext.consensusLogger = logger;
      anotherBlockExecutionContext.withdrawalTransactionsMap = plainObject
        .withdrawalTransactionsMap;
      anotherBlockExecutionContext.previousCoreChainLockedHeight = previousCoreChainLockedHeight;
      anotherBlockExecutionContext.previousHeight = previousHeight;
      anotherBlockExecutionContext.previousBlockTime = previousTime;

      blockExecutionContext.populate(anotherBlockExecutionContext);

      expect(blockExecutionContext.dataContracts).to.equal(
        anotherBlockExecutionContext.dataContracts,
      );
      expect(blockExecutionContext.lastCommitInfo).to.equal(
        anotherBlockExecutionContext.lastCommitInfo,
      );
      expect(blockExecutionContext.cumulativeProcessingFee).to.equal(
        anotherBlockExecutionContext.cumulativeProcessingFee,
      );
      expect(blockExecutionContext.cumulativeStorageFee).to.equal(
        anotherBlockExecutionContext.cumulativeStorageFee,
      );
      expect(blockExecutionContext.height).to.equal(
        anotherBlockExecutionContext.height,
      );
      expect(blockExecutionContext.time).to.equal(
        anotherBlockExecutionContext.time,
      );
      expect(blockExecutionContext.version).to.equal(
        anotherBlockExecutionContext.version,
      );
      expect(blockExecutionContext.coreChainLockedHeight).to.equal(
        anotherBlockExecutionContext.coreChainLockedHeight,
      );
      expect(blockExecutionContext.consensusLogger).to.equal(
        anotherBlockExecutionContext.consensusLogger,
      );
      expect(blockExecutionContext.withdrawalTransactionsMap).to.equal(
        anotherBlockExecutionContext.withdrawalTransactionsMap,
      );
      expect(blockExecutionContext.previousHeight).to.equal(
        anotherBlockExecutionContext.previousHeight,
      );
      expect(blockExecutionContext.previousCoreChainLockedHeight).to.equal(
        anotherBlockExecutionContext.previousCoreChainLockedHeight,
      );
      expect(blockExecutionContext.previousBlockTime).to.equal(
        anotherBlockExecutionContext.previousBlockTime,
      );
    });
  });

  describe('#toObject', () => {
    it('should return a plain object', () => {
      blockExecutionContext.dataContracts = [dataContract];
      blockExecutionContext.lastCommitInfo = lastCommitInfo;
      blockExecutionContext.cumulativeProcessingFee = cumulativeProcessingFee;
      blockExecutionContext.cumulativeStorageFee = cumulativeStorageFee;
      blockExecutionContext.height = height;
      blockExecutionContext.coreChainLockedHeight = coreChainLockedHeight;
      blockExecutionContext.time = time;
      blockExecutionContext.version = version;
      blockExecutionContext.consensusLogger = logger;
      blockExecutionContext.withdrawalTransactionsMap = plainObject.withdrawalTransactionsMap;
      blockExecutionContext.previousBlockTime = previousTime;
      blockExecutionContext.previousHeight = previousHeight;
      blockExecutionContext.previousCoreChainLockedHeight = previousCoreChainLockedHeight;

      expect(blockExecutionContext.toObject()).to.deep.equal(plainObject);
    });

    it('should skipConsensusLogger if the option passed', () => {
      blockExecutionContext.dataContracts = [dataContract];
      blockExecutionContext.lastCommitInfo = lastCommitInfo;
      blockExecutionContext.cumulativeProcessingFee = cumulativeProcessingFee;
      blockExecutionContext.cumulativeStorageFee = cumulativeStorageFee;
      blockExecutionContext.height = height;
      blockExecutionContext.coreChainLockedHeight = coreChainLockedHeight;
      blockExecutionContext.time = time;
      blockExecutionContext.time.seconds = time.seconds;
      blockExecutionContext.version = version;
      blockExecutionContext.consensusLogger = logger;
      blockExecutionContext.withdrawalTransactionsMap = plainObject.withdrawalTransactionsMap;
      blockExecutionContext.previousBlockTime = previousTime;
      blockExecutionContext.previousHeight = previousHeight;
      blockExecutionContext.previousCoreChainLockedHeight = previousCoreChainLockedHeight;

      const result = blockExecutionContext.toObject({ skipConsensusLogger: true });

      delete plainObject.consensusLogger;

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
      expect(blockExecutionContext.cumulativeProcessingFee).to.equal(cumulativeProcessingFee);
      expect(blockExecutionContext.cumulativeStorageFee).to.equal(cumulativeStorageFee);
      expect(blockExecutionContext.height).to.deep.equal(height);
      expect(blockExecutionContext.version).to.deep.equal(version);
      expect(blockExecutionContext.time).to.deep.equal(time);
      expect(blockExecutionContext.coreChainLockedHeight).to.deep.equal(coreChainLockedHeight);
      expect(blockExecutionContext.consensusLogger).to.equal(logger);
      expect(blockExecutionContext.withdrawalTransactionsMap).to.deep.equal(
        plainObject.withdrawalTransactionsMap,
      );
      expect(blockExecutionContext.previousBlockTime).to.deep.equal(
        previousTime,
      );
      expect(blockExecutionContext.previousHeight).to.deep.equal(
        previousHeight,
      );
      expect(blockExecutionContext.previousCoreChainLockedHeight).to.deep.equal(
        previousCoreChainLockedHeight,
      );
    });
  });
});
