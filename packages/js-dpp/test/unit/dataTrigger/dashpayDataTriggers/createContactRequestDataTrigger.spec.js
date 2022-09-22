const createContactRequestDataTrigger = require('../../../../lib/dataTrigger/dashpayDataTriggers/createContactRequestDataTrigger');

const DataTriggerExecutionContext = require('../../../../lib/dataTrigger/DataTriggerExecutionContext');
const DataTriggerExecutionResult = require('../../../../lib/dataTrigger/DataTriggerExecutionResult');
const DataTriggerConditionError = require('../../../../lib/errors/consensus/state/dataContract/dataTrigger/DataTriggerConditionError');

const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');
const getDocumentTransitionFixture = require('../../../../lib/test/fixtures/getDocumentTransitionsFixture');

const getDashPayContractFixture = require('../../../../lib/test/fixtures/getDashPayContractFixture');
const { getContactRequestDocumentFixture } = require('../../../../lib/test/fixtures/getDashPayDocumentFixture');
const StateTransitionExecutionContext = require('../../../../lib/stateTransition/StateTransitionExecutionContext');

describe('createContactRequestDataTrigger', () => {
  let context;
  let dashPayIdentity;
  let stateRepositoryMock;
  let dataContract;
  let contactRequestDocument;
  let documentTransition;

  beforeEach(function beforeEach() {
    contactRequestDocument = getContactRequestDocumentFixture();
    dataContract = getDashPayContractFixture();

    [documentTransition] = getDocumentTransitionFixture({
      create: [contactRequestDocument],
    });

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight.resolves(42);

    context = new DataTriggerExecutionContext(
      stateRepositoryMock,
      contactRequestDocument.getOwnerId(),
      dataContract,
      new StateTransitionExecutionContext(),
    );

    dashPayIdentity = context.getOwnerId();
  });

  it('should successfully execute if document is valid', async () => {
    contactRequestDocument.data.coreHeightCreatedAt = 40;

    [documentTransition] = getDocumentTransitionFixture({
      create: [contactRequestDocument],
    });

    const result = await createContactRequestDataTrigger(
      documentTransition, context, dashPayIdentity,
    );

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.be.calledOnce();
    expect(result.isOk()).to.be.true();
  });

  it('should successfully execute if document has no `coreHeightCreatedAt` field', async () => {
    const result = await createContactRequestDataTrigger(
      documentTransition, context, dashPayIdentity,
    );

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.be.not.called();
    expect(result.isOk()).to.be.true();
  });

  it('should fail with out of window error', async () => {
    contactRequestDocument.data.coreHeightCreatedAt = 10;

    [documentTransition] = getDocumentTransitionFixture({
      create: [contactRequestDocument],
    });

    const result = await createContactRequestDataTrigger(
      documentTransition, context, dashPayIdentity,
    );

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataTriggerConditionError);
    expect(error.message).to.equal('Core height 10 is out of block height window from 34 to 50');
  });

  it('should successfully execute on dry run', async () => {
    contactRequestDocument.data.coreHeightCreatedAt = 10;
    [documentTransition] = getDocumentTransitionFixture({
      create: [contactRequestDocument],
    });

    context.getStateTransitionExecutionContext().enableDryRun();

    const result = await createContactRequestDataTrigger(
      documentTransition, context, dashPayIdentity,
    );

    context.getStateTransitionExecutionContext().disableDryRun();

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.be.not.called();
    expect(result.isOk()).to.be.true();
  });
});
