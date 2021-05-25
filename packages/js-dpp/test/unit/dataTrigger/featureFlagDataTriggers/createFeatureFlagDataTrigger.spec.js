const Long = require('long');

const createFeatureFlagDataTrigger = require('../../../../lib/dataTrigger/featureFlagsDataTriggers/createFeatureFlagDataTrigger');

const getIdentityFixture = require('../../../../lib/test/fixtures/getIdentityFixture');
const getFeatureFlagsDocumentsFixture = require('../../../../lib/test/fixtures/getFeatureFlagsDocumentsFixture');
const getDocumentTransitionsFixture = require('../../../../lib/test/fixtures/getDocumentTransitionsFixture');
const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');
const DataTriggerExecutionResult = require('../../../../lib/dataTrigger/DataTriggerExecutionResult');
const DataTriggerConditionError = require('../../../../lib/errors/DataTriggerConditionError');
const Identifier = require('../../../../lib/identifier/Identifier');

describe('createFeatureFlagDataTrigger', () => {
  let contextMock;
  let stateRepositoryMock;
  let documentTransition;
  let topLevelIdentityId;

  beforeEach(function beforeEach() {
    topLevelIdentityId = getIdentityFixture().getId();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchLatestPlatformBlockHeader.resolves({
      height: new Long(42),
    });

    const [document] = getFeatureFlagsDocumentsFixture();

    [documentTransition] = getDocumentTransitionsFixture({
      create: [document],
    });

    contextMock = {
      getStateRepository: () => stateRepositoryMock,
      getOwnerId: this.sinonSandbox.stub(),
      getDataContract: () => getFeatureFlagsDocumentsFixture.dataContract,
    };
    contextMock.getOwnerId.returns(topLevelIdentityId);
  });

  it('should return an error if heigh is lower than block height', async () => {
    documentTransition.data.enableAtHeight = 1;

    const result = await createFeatureFlagDataTrigger(
      documentTransition, contextMock, topLevelIdentityId,
    );

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataTriggerConditionError);
    expect(error.message).to.equal('Feature flag cannot be enabled in the past');
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
});
