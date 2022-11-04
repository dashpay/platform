const Long = require('long');

const createFeatureFlagDataTrigger = require('@dashevo/dpp/lib/dataTrigger/featureFlagsDataTriggers/createFeatureFlagDataTrigger');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const getFeatureFlagsDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getFeatureFlagsDocumentsFixture');
const getDocumentTransitionsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentTransitionsFixture');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const DataTriggerExecutionResult = require('@dashevo/dpp/lib/dataTrigger/DataTriggerExecutionResult');
const DataTriggerConditionError = require('@dashevo/dpp/lib/errors/consensus/state/dataContract/dataTrigger/DataTriggerConditionError');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const StateTransitionExecutionContext = require('@dashevo/dpp/lib/stateTransition/StateTransitionExecutionContext');

describe('createFeatureFlagDataTrigger', () => {
  let contextMock;
  let stateRepositoryMock;
  let documentTransition;
  let topLevelIdentityId;

  beforeEach(function beforeEach() {
    topLevelIdentityId = getIdentityFixture().getId();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchLatestPlatformBlockHeight.resolves(new Long(42));

    const [document] = getFeatureFlagsDocumentsFixture();

    [documentTransition] = getDocumentTransitionsFixture({
      create: [document],
    });

    const context = new StateTransitionExecutionContext();

    contextMock = {
      getStateRepository: () => stateRepositoryMock,
      getOwnerId: this.sinonSandbox.stub(),
      getDataContract: () => getFeatureFlagsDocumentsFixture.dataContract,
      getStateTransitionExecutionContext: () => context,
    };
    contextMock.getOwnerId.returns(topLevelIdentityId);
  });

  it('should return an error if height is lower than block height', async () => {
    documentTransition.data.enableAtHeight = 1;

    const result = await createFeatureFlagDataTrigger(
      documentTransition, contextMock, topLevelIdentityId,
    );

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataTriggerConditionError);
    expect(error.message).to.equal('Feature flag cannot be enabled in the past on block 1. Current block height is 42');
  });

  it('should return an error if owner id is not equal to top level identity id', async () => {
    contextMock.getOwnerId.returns(Identifier.from(Buffer.alloc(32, 1)));

    const result = await createFeatureFlagDataTrigger(
      documentTransition, contextMock, topLevelIdentityId,
    );

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataTriggerConditionError);
    expect(error.message).to.equal('This identity can\'t activate selected feature flag');
  });

  it('should pass', async () => {
    const result = await createFeatureFlagDataTrigger(
      documentTransition, contextMock, topLevelIdentityId,
    );

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.true();
  });

  it('should pass on dry run', async () => {
    contextMock.getStateTransitionExecutionContext().enableDryRun();

    const result = await createFeatureFlagDataTrigger(
      documentTransition, contextMock, topLevelIdentityId,
    );

    contextMock.getStateTransitionExecutionContext().disableDryRun();

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.true();

    expect(contextMock.getOwnerId).to.not.be.called();
    expect(stateRepositoryMock.fetchLatestPlatformBlockHeight).to.not.be.called();
  });
});
