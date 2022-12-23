const bs58 = require('bs58');
const rejectDataTrigger = require('@dashevo/dpp/lib/dataTrigger/rejectDataTrigger');

const DataTriggerExecutionContext = require('@dashevo/dpp/lib/dataTrigger/DataTriggerExecutionContext');

const { getChildDocumentFixture } = require('@dashevo/dpp/lib/test/fixtures/getDpnsDocumentFixture');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const getDpnsContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDpnsContractFixture');
const getDocumentTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentTransitionsFixture');

const DataTriggerExecutionResult = require('@dashevo/dpp/lib/dataTrigger/DataTriggerExecutionResult');
const StateTransitionExecutionContext = require('@dashevo/dpp/lib/stateTransition/StateTransitionExecutionContext');

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
