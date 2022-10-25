const {
  tendermint: {
    types: {
      ConsensusParams,
    },
  },
} = require('@dashevo/abci/types');
const Long = require('long');
const updateConsensusParamsFactory = require('../../../../../lib/abci/handlers/proposal/updateConsensusParamsFactory');
const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');
const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');

describe('updateConsensusParamsFactory', () => {
  let updateConsensusParams;
  let getFeatureFlagForHeightMock;
  let blockExecutionContextMock;
  let loggerMock;
  let height;
  let version;
  let getLatestFeatureFlagGetMock;

  beforeEach(function beforeEach() {
    loggerMock = new LoggerMock(this.sinon);
    height = Long.fromInt(15);
    version = {
      app: Long.fromInt(1),
    };

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    blockExecutionContextMock.getVersion.returns(version);

    getFeatureFlagForHeightMock = this.sinon.stub().resolves(null);
    getLatestFeatureFlagGetMock = this.sinon.stub();

    updateConsensusParams = updateConsensusParamsFactory(
      blockExecutionContextMock,
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

    const response = await updateConsensusParams(height, loggerMock);

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

    const response = await updateConsensusParams(height, loggerMock);

    expect(response).to.be.undefined();

    expect(getFeatureFlagForHeightMock).to.be.calledOnce();
  });
});
