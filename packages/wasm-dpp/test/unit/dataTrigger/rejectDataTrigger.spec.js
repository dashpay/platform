const bs58 = require('bs58');
const rejectDataTrigger = require('../../../lib/dataTrigger/rejectDataTrigger');

const DataTriggerExecutionContext = require('../../../lib/dataTrigger/DataTriggerExecutionContext');

const { getChildDocumentFixture } = require('../../../lib/test/fixtures/getDpnsDocumentFixture');

const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');

const getDpnsContractFixture = require('../../../lib/test/fixtures/getDpnsContractFixture');
const getDocumentTransitionFixture = require('../../../lib/test/fixtures/getDocumentTransitionsFixture');

const DataTriggerExecutionResult = require('../../../lib/dataTrigger/DataTriggerExecutionResult');
const StateTransitionExecutionContext = require('../../../lib/stateTransition/StateTransitionExecutionContext');

describe('rejectDataTrigger', () => {
  let documentTransition;
  let context;
  let stateRepositoryMock;
  let dataContract;

  beforeEach(function beforeEach() {
    dataContract = getDpnsContractFixture();
    const document = getChildDocumentFixture();

    [documentTransition] = getDocumentTransitionFixture({
      create: [],
      delete: [document],
    });

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    context = new DataTriggerExecutionContext(
      stateRepositoryMock,
      bs58.decode('5zcXZpTLWFwZjKjq3ME5KVavtZa9YUaZESVzrndehBhq'),
      dataContract,
      new StateTransitionExecutionContext(),
    );
  });

  it('should always fail', async () => {
    const result = await rejectDataTrigger(documentTransition, context);

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);

    expect(result.isOk()).to.be.false();

    const [error] = result.getErrors();

    expect(error.message).to.equal('Action is not allowed');
  });
});
