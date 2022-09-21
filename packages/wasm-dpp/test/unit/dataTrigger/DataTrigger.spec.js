const bs58 = require('bs58');
const AbstractDocumentTransition = require('../../../lib/document/stateTransition/DocumentsBatchTransition/documentTransition/AbstractDocumentTransition');
const DataTrigger = require('../../../lib/dataTrigger/DataTrigger');
const DataTriggerExecutionContext = require('../../../lib/dataTrigger/DataTriggerExecutionContext');
const getDpnsContractFixture = require('../../../lib/test/fixtures/getDpnsContractFixture');
const DataTriggerExecutionResult = require('../../../lib/dataTrigger/DataTriggerExecutionResult');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const DataTriggerExecutionError = require('../../../lib/errors/consensus/state/dataContract/dataTrigger/DataTriggerExecutionError');
const DataTriggerInvalidResultError = require('../../../lib/errors/consensus/state/dataContract/dataTrigger/DataTriggerInvalidResultError');

describe('DataTrigger', () => {
  let dataContractMock;
  let context;
  let triggerStub;
  let document;
  let topLevelIdentity;

  beforeEach(function beforeEach() {
    triggerStub = this.sinonSandbox.stub().resolves(new DataTriggerExecutionResult());
    dataContractMock = getDpnsContractFixture();

    ([document] = getDocumentsFixture());

    context = new DataTriggerExecutionContext(
      null,
      bs58.decode('5zcXZpTLWFwZjKjq3ME5KVavtZa9YUaZESVzrndehBhq'),
      dataContractMock,
    );

    topLevelIdentity = context.getOwnerId();
  });

  it('should check trigger fields', () => {
    const trigger = new DataTrigger(
      dataContractMock.getId(),
      document.getType(),
      AbstractDocumentTransition.ACTIONS.CREATE,
      triggerStub,
      topLevelIdentity,
    );

    expect(trigger.dataContractId).to.equal(dataContractMock.getId());
    expect(trigger.documentType).to.equal(document.getType());
    expect(trigger.transitionAction).to.equal(AbstractDocumentTransition.ACTIONS.CREATE);
    expect(trigger.trigger).to.equal(triggerStub);
    expect(trigger.topLevelIdentity).to.equal(topLevelIdentity);
  });

  describe('#execute', () => {
    it('should check trigger execution', async () => {
      const trigger = new DataTrigger(
        dataContractMock.getId(),
        document.getType(),
        AbstractDocumentTransition.ACTIONS.CREATE,
        triggerStub,
        topLevelIdentity,
      );

      const result = await trigger.execute(context);

      expect(result).to.be.instanceOf(DataTriggerExecutionResult);
    });

    it('should pass through the result of the trigger function', async () => {
      const functionResult = new DataTriggerExecutionResult();

      const triggerError = new Error('Trigger error');

      functionResult.addError(triggerError);

      triggerStub.resolves(functionResult);

      const trigger = new DataTrigger(
        dataContractMock.getId(),
        document.getType(),
        AbstractDocumentTransition.ACTIONS.CREATE,
        triggerStub,
        topLevelIdentity,
      );

      const result = await trigger.execute(document, context);

      expect(result).to.deep.equal(functionResult);
      expect(result.getErrors()[0]).to.deep.equal(triggerError);
    });

    it('should return a result with execution error if trigger function have thrown an error', async () => {
      const triggerError = new Error('Trigger error');

      triggerStub.throws(triggerError);

      const trigger = new DataTrigger(
        dataContractMock.getId(),
        document.getType(),
        AbstractDocumentTransition.ACTIONS.CREATE,
        triggerStub,
        topLevelIdentity,
      );

      const result = await trigger.execute(document, context);

      expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(DataTriggerExecutionError);

      expect(error.getExecutionError()).to.equal(triggerError);
    });

    it('should return a result with invalid result error if trigger function have not returned any result', async () => {
      triggerStub.resolves(null);

      const trigger = new DataTrigger(
        dataContractMock.getId(),
        document.getType(),
        AbstractDocumentTransition.ACTIONS.CREATE,
        triggerStub,
        topLevelIdentity,
      );

      const result = await trigger.execute(document, context);

      expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(DataTriggerInvalidResultError);
      expect(error.message).to.equal('Data trigger have not returned any result');
    });
  });
});
