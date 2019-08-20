const Document = require('../../../../lib/document/Document');
const DataTrigger = require('../../../../lib/dataTrigger/DataTrigger');
const DataTriggerExecutionResult = require('../../../../lib/dataTrigger/DataTriggerExecutionResult');
const DataTriggerExecutionContext = require('../../../../lib/dataTrigger/DataTriggerExecutionContext');
const getDpnsContractFixture = require('../../../../lib/test/fixtures/getDpnsContractFixture');
const dpnsDocumentFixture = require('../../../../lib/test/fixtures/getDpnsDocumentFixture');
const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');

const dpnsCreateDomainDataTrigger = require('../../../../lib/dataTrigger/dpnsTriggers/createDomainDataTrigger');
const dpnsDeleteDomainDataTrigger = require('../../../../lib/dataTrigger/dpnsTriggers/createDomainDataTrigger');
const dpnsUpdateDomainDataTrigger = require('../../../../lib/dataTrigger/dpnsTriggers/createDomainDataTrigger');

const executeDataTriggersFactory = require('../../../../lib/stPacket/verification/executeDataTriggersFactory');

describe('executeDataTriggersFactory', () => {
  let childDocument;
  let contractMock;

  let dpnsTriggers;

  let domainDocumentType;

  let stateTransitionHeaderMock;
  let context;
  let documents;
  let dpnsCreateDomainDataTriggerMock;
  let dpnsUpdateDomainDataTriggerMock;
  let dpnsDeleteDomainDataTriggerMock;
  let getDataTriggersMock;

  let executeDataTriggers;

  beforeEach(function beforeEach() {
    domainDocumentType = 'domain';

    dpnsTriggers = [
      dpnsCreateDomainDataTrigger,
      dpnsDeleteDomainDataTrigger,
      dpnsUpdateDomainDataTrigger,
    ];

    contractMock = getDpnsContractFixture();

    childDocument = dpnsDocumentFixture.getChildDocumentFixture();

    dpnsCreateDomainDataTriggerMock = { execute: this.sinonSandbox.stub() };
    dpnsUpdateDomainDataTriggerMock = { execute: this.sinonSandbox.stub() };
    dpnsDeleteDomainDataTriggerMock = { execute: this.sinonSandbox.stub() };

    dpnsCreateDomainDataTriggerMock
      .execute.resolves(new DataTriggerExecutionResult());

    dpnsUpdateDomainDataTriggerMock
      .execute.resolves(new DataTriggerExecutionResult());

    dpnsDeleteDomainDataTriggerMock
      .execute.resolves(new DataTriggerExecutionResult());

    const userId = 'userId';

    context = new DataTriggerExecutionContext(
      null, userId, contractMock, stateTransitionHeaderMock,
    );

    documents = [childDocument];

    getDataTriggersMock = this.sinonSandbox.stub();

    getDataTriggersMock.returns([
      dpnsCreateDomainDataTriggerMock,
    ]);

    executeDataTriggers = executeDataTriggersFactory(getDataTriggersMock);
  });

  it('should return an array of DataTriggerExecutionResult', async () => {
    const dataTriggerExecutionResults = await executeDataTriggers(
      documents, context,
    );

    expect(dataTriggerExecutionResults).to.have.a.lengthOf(1);

    const [result] = dataTriggerExecutionResults;

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.getErrors()).to.have.a.lengthOf(0);
    expect(result.isOk()).to.be.true();
  });

  it('should execute multiple data triggers if there is more than one data trigger for'
    + ' the same document and action in the contract', async () => {
    getDataTriggersMock.returns([
      dpnsCreateDomainDataTriggerMock,
      dpnsCreateDomainDataTriggerMock,
      dpnsCreateDomainDataTriggerMock,
    ]);

    const expectedTriggersCount = 3;
    expect(dpnsTriggers.length).to.equal(expectedTriggersCount);

    const dataTriggerExecutionResults = await executeDataTriggers(
      documents, context,
    );

    expect(dataTriggerExecutionResults).to.have.a.lengthOf(expectedTriggersCount);

    dataTriggerExecutionResults.forEach((dataTriggerExecutionResult) => {
      expect(dataTriggerExecutionResult.getErrors()).to.have.a.lengthOf(0);
    });
  });

  it('should return a result for each passed document with success or error', async function test() {
    const doc1 = getDocumentsFixture()[0];
    const doc2 = getDocumentsFixture()[1];

    documents = [doc1, doc2, doc1];

    const passingExecutionResult = new DataTriggerExecutionResult();
    const executionResultWithErrors = new DataTriggerExecutionResult();

    executionResultWithErrors.addError(new Error('Trigger error'));

    const passingTriggerMockFunction = this.sinonSandbox.stub()
      .resolves(passingExecutionResult);
    const throwingTriggerMockFunction = this.sinonSandbox.stub()
      .resolves(executionResultWithErrors);

    const passingDataTriggerMock = new DataTrigger(
      contractMock.getId(),
      doc1.getType(),
      doc1.getAction(),
      passingTriggerMockFunction,
    );

    const throwingDataTriggerMock = new DataTrigger(
      contractMock.getId(),
      doc2.getType(),
      doc2.getAction(),
      throwingTriggerMockFunction,
    );

    getDataTriggersMock
      .withArgs(contractMock.getId(), doc1.getType(), doc1.getAction())
      .returns([passingDataTriggerMock]);

    getDataTriggersMock
      .withArgs(contractMock.getId(), doc2.getType(), doc2.getAction())
      .returns([throwingDataTriggerMock]);

    context = new DataTriggerExecutionContext(
      null, 'id', contractMock, stateTransitionHeaderMock,
    );

    const dataTriggerExecutionResults = await executeDataTriggers(
      documents, context,
    );

    const expectedResultsCount = 3;

    expect(documents.length).to.equal(expectedResultsCount);
    expect(dataTriggerExecutionResults.length).to.equal(expectedResultsCount);

    const passingResults = dataTriggerExecutionResults.filter(result => result.isOk());
    const failingResults = dataTriggerExecutionResults.filter(result => !result.isOk());

    expect(passingResults).to.have.a.lengthOf(2);
    expect(failingResults).to.have.a.lengthOf(1);

    expect(failingResults[0].getErrors()).to.have.a.lengthOf(1);
    expect(failingResults[0].getErrors()[0].message).to
      .equal('Trigger error');

    expect(passingTriggerMockFunction.callCount).to.equal(2);
    expect(throwingTriggerMockFunction.callCount).to.equal(1);
  });

  it("should not call any triggers if documents have no triggers associated with it's type or action", async () => {
    getDataTriggersMock
      .withArgs(contractMock.getId(), domainDocumentType, Document.ACTIONS.CREATE)
      .returns([])
      .withArgs(contractMock.getId(), domainDocumentType, Document.ACTIONS.DELETE)
      .returns([dpnsDeleteDomainDataTriggerMock])
      .withArgs(contractMock.getId(), domainDocumentType, Document.ACTIONS.UPDATE)
      .returns([dpnsUpdateDomainDataTriggerMock]);

    await executeDataTriggers(documents, context);

    expect(dpnsDeleteDomainDataTriggerMock.execute).not.to.be.called();
    expect(dpnsUpdateDomainDataTriggerMock.execute).not.to.be.called();
  });

  it("should call only one trigger if there's one document with a trigger and one without", async () => {
    documents = [childDocument].concat(getDocumentsFixture());

    getDataTriggersMock.resetBehavior();
    getDataTriggersMock
      .returns([])
      .withArgs(contractMock.getId(), domainDocumentType, Document.ACTIONS.CREATE)
      .returns([dpnsCreateDomainDataTriggerMock])
      .withArgs(contractMock.getId(), domainDocumentType, Document.ACTIONS.DELETE)
      .returns([dpnsDeleteDomainDataTriggerMock])
      .withArgs(contractMock.getId(), domainDocumentType, Document.ACTIONS.UPDATE)
      .returns([dpnsUpdateDomainDataTriggerMock]);

    await executeDataTriggers(documents, context);

    expect(dpnsCreateDomainDataTriggerMock.execute).to.be.calledOnce();
    expect(dpnsDeleteDomainDataTriggerMock.execute).not.to.be.called();
    expect(dpnsUpdateDomainDataTriggerMock.execute).not.to.be.called();
  });

  it("should not call any triggers if there's no triggers in the contract", async () => {
    documents = getDocumentsFixture();

    getDataTriggersMock.resetBehavior();
    getDataTriggersMock
      .returns([])
      .withArgs(contractMock.getId(), domainDocumentType, Document.ACTIONS.CREATE)
      .returns([dpnsCreateDomainDataTriggerMock])
      .withArgs(contractMock.getId(), domainDocumentType, Document.ACTIONS.DELETE)
      .returns([dpnsDeleteDomainDataTriggerMock])
      .withArgs(contractMock.getId(), domainDocumentType, Document.ACTIONS.UPDATE)
      .returns([dpnsUpdateDomainDataTriggerMock]);

    await executeDataTriggers(documents, context);

    expect(dpnsCreateDomainDataTriggerMock.execute).not.to.be.called();
    expect(dpnsDeleteDomainDataTriggerMock.execute).not.to.be.called();
    expect(dpnsUpdateDomainDataTriggerMock.execute).not.to.be.called();
  });
});
