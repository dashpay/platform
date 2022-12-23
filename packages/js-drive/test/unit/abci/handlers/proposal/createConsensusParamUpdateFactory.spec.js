const {
  tendermint: {
    types: {
      ConsensusParams,
    },
  },
} = require('@dashevo/abci/types');
const Long = require('long');
const createConsensusParamUpdateFactory = require('../../../../../lib/abci/handlers/proposal/createConsensusParamUpdateFactory');
const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');
const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');

describe('createConsensusParamUpdateFactory', () => {
  let createConsensusParamUpdate;
  let getFeatureFlagForHeightMock;
  let loggerMock;
  let height;
  let version;
  let getLatestFeatureFlagGetMock;
  let proposalBlockExecutionContextMock;
  let round;

  beforeEach(function beforeEach() {
    round = 42;
    loggerMock = new LoggerMock(this.sinon);
    height = Long.fromInt(15);
    version = {
      app: Long.fromInt(1),
    };

    proposalBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    proposalBlockExecutionContextMock.getVersion.returns(version);

    getFeatureFlagForHeightMock = this.sinon.stub().resolves(null);
    getLatestFeatureFlagGetMock = this.sinon.stub();

    createConsensusParamUpdate = createConsensusParamUpdateFactory(
      proposalBlockExecutionContextMock,
      getFeatureFlagForHeightMock,
    );
  });

  it('should return consensusParamUpdates if request contains update consensus features flag', async () => {
    getLatestFeatureFlagGetMock.withArgs('block').returns({
      maxBytes: 1,
      maxGas: 2,
    });
    getLatestFeatureFlagGetMock.withArgs('evidence').returns({
      maxAgeNumBlocks: 1,
      maxAgeDuration: null,
      maxBytes: 2,
    });
    getLatestFeatureFlagGetMock.withArgs('version').returns({
      appVersion: 1,
    });

    getFeatureFlagForHeightMock.resolves({
      get: getLatestFeatureFlagGetMock,
    });

    const response = await createConsensusParamUpdate(height, round, loggerMock);

    expect(response).to.deep.equal(new ConsensusParams({
      block: {
        maxBytes: 1,
        maxGas: 2,
      },
      evidence: {
        maxAgeDuration: null,
        maxAgeNumBlocks: 1,
        maxBytes: 2,
      },
      version: {
        appVersion: 1,
      },
    }));

    expect(getFeatureFlagForHeightMock).to.be.calledOnce();
  });

  it('should return undefined', async () => {
    getFeatureFlagForHeightMock.resolves(null);

    const response = await createConsensusParamUpdate(height, round, loggerMock);

    expect(response).to.be.undefined();

    expect(getFeatureFlagForHeightMock).to.be.calledOnce();
  });
});
