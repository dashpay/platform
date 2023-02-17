const bs58 = require('bs58');
const AbstractDocumentTransition = require('@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/documentTransition/AbstractDocumentTransition');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const dpnsSystemIds = require('@dashevo/dpns-contract/lib/systemIds');
const DataTriggerJs = require('@dashevo/dpp/lib/dataTrigger/DataTrigger');
const DataTriggerExecutionResultJs = require('@dashevo/dpp/lib/dataTrigger/DataTriggerExecutionResult');
const DataTriggerExecutionContextJs = require('@dashevo/dpp/lib/dataTrigger/DataTriggerExecutionContext');
const getDpnsContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDpnsContractFixture');
const dpnsDocumentFixture = require('@dashevo/dpp/lib/test/fixtures/getDpnsDocumentFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentTransitionsFixture');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const dpnsCreateDomainDataTriggerJs = require('@dashevo/dpp/lib/dataTrigger/dpnsTriggers/createDomainDataTrigger');
const dpnsDeleteDomainDataTriggerJs = require('@dashevo/dpp/lib/dataTrigger/dpnsTriggers/createDomainDataTrigger');
const dpnsUpdateDomainDataTriggerJs = require('@dashevo/dpp/lib/dataTrigger/dpnsTriggers/createDomainDataTrigger');

const executeDataTriggersFactory = require('@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/validation/state/executeDataTriggersFactory');

const IdentifierJs = require('@dashevo/dpp/lib/identifier/Identifier');

const { default: loadWasmDpp } = require('../../../../../../../dist');

let DataContract;
let DocumentTransition;
let DocumentCreateTransition;
let DataTriggerExecutionContext;
let Document;
let DataTriggerExecutionResult;
let StateTransitionExecutionContext;
let getAllDataTriggers;

describe('executeDataTriggersFactory', () => {
  let childDocumentJs;
  let childDocument;
  let contractMockJs;
  let contractMock;
  let dpnsTriggers;
  let domainDocumentType;
  let stateTransitionHeaderMock;
  let stateTransitionExecutionContext;
  let contextJs;
  let context;
  let documentTransitionsJs;
  let documentTransitions;
  let dpnsCreateDomainDataTriggerMock;
  let dpnsUpdateDomainDataTriggerMock;
  let dpnsDeleteDomainDataTriggerMock;
  let getDataTriggersMock;

  let executeDataTriggersJs;
  let executeDataTriggers;
  let dataContractJs;
  let dataContract;
  let stateRepositoryMock;

  beforeEach(async function beforeEach() {
    ({
      DataContract,
      DocumentTransition,
      DocumentCreateTransition,
      DataTriggerExecutionContext,
      Document,
      DataTriggerExecutionResult,
      StateTransitionExecutionContext,
      executeDataTriggers,
      getAllDataTriggers,
    } = await loadWasmDpp());

    dataContractJs = getDataContractFixture();
    dataContract = new DataContract(dataContractJs.toObject());

    domainDocumentType = 'domain';

    dpnsTriggers = [
      dpnsCreateDomainDataTriggerJs,
      dpnsDeleteDomainDataTriggerJs,
      dpnsUpdateDomainDataTriggerJs,
    ];

    contractMockJs = getDpnsContractFixture();
    contractMock = new DataContract(contractMockJs.toObject());

    childDocumentJs = dpnsDocumentFixture.getChildDocumentFixture();
    childDocument = new Document(childDocumentJs.toObject(), dataContract.clone());

    dpnsCreateDomainDataTriggerMock = { execute: this.sinonSandbox.stub() };
    dpnsUpdateDomainDataTriggerMock = { execute: this.sinonSandbox.stub() };
    dpnsDeleteDomainDataTriggerMock = { execute: this.sinonSandbox.stub() };

    dpnsCreateDomainDataTriggerMock
      .execute.returns(new DataTriggerExecutionResultJs());

    dpnsUpdateDomainDataTriggerMock
      .execute.returns(new DataTriggerExecutionResultJs());

    dpnsDeleteDomainDataTriggerMock
      .execute.returns(new DataTriggerExecutionResultJs());

    const ownerId = bs58.decode('5zcXZpTLWFwZjKjq3ME5KVavtZa9YUaZESVzrndehBhq');

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    contextJs = new DataTriggerExecutionContextJs(
      null, ownerId, contractMockJs,
    );

    contractMock.setId(dpnsSystemIds.contractId);
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDocuments.resolves([childDocument]);
    stateTransitionExecutionContext = new StateTransitionExecutionContext();
    stateTransitionExecutionContext.disableDryRun();

    context = new DataTriggerExecutionContext(stateRepositoryMock, childDocument.getOwnerId(), contractMock.clone(), stateTransitionExecutionContext);

    documentTransitionsJs = getDocumentTransitionsFixture({
      create: [childDocumentJs],
    });

    documentTransitions = documentTransitionsJs.map((transition) => {
      const transitionCreate = new DocumentCreateTransition(transition.toObject(), dataContract.clone());
      return DocumentTransition.fromTransitionCreate(transitionCreate);
    });

    getDataTriggersMock = this.sinonSandbox.stub();

    getDataTriggersMock.returns([
      dpnsCreateDomainDataTriggerMock,
    ]);

    executeDataTriggersJs = executeDataTriggersFactory(getDataTriggersMock);
  });

  it('should return an array of DataTriggerExecutionResult', async () => {
    const dataTriggerExecutionResults = await executeDataTriggersJs(
      documentTransitionsJs, contextJs,
    );

    expect(dataTriggerExecutionResults).to.have.a.lengthOf(1);

    const [result] = dataTriggerExecutionResults;

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResultJs);
    expect(result.getErrors()).to.have.a.lengthOf(0);
    expect(result.isOk()).to.be.true();
  });

  it('should return an array of DataTriggerExecutionResult - Rust', async () => {
    const dataTriggerExecutionResults = await executeDataTriggers(
      documentTransitions, context, getAllDataTriggers(),
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

      const dataTriggerExecutionResults = await executeDataTriggersJs(
        documentTransitionsJs, contextJs,
      );

      expect(dataTriggerExecutionResults).to.have.a.lengthOf(expectedTriggersCount);

      dataTriggerExecutionResults.forEach((dataTriggerExecutionResult) => {
        expect(dataTriggerExecutionResult.getErrors()).to.have.a.lengthOf(0);
      });
    });

  it('should return multiple data triggers if there is more than one data trigger for'
    + ' the same document and action in the contract - Rust', async () => {
      const dataTriggersList = getAllDataTriggers();
      const dataTriggerListWithDuplicates = [dataTriggersList[0], dataTriggersList[0], dataTriggersList[0]];

      const expectedTriggersCount = 3;
      expect(dpnsTriggers.length).to.equal(expectedTriggersCount);

      const dataTriggerExecutionResults = await executeDataTriggers(
        documentTransitions, context, dataTriggerListWithDuplicates
      );

      expect(dataTriggerExecutionResults).to.have.a.lengthOf(expectedTriggersCount);
    });

  it('should return a result for each passed document with success or error', async function test() {
    const doc1 = getDocumentsFixture(dataContractJs)[0];
    const doc2 = getDocumentsFixture(dataContractJs)[1];

    documentTransitionsJs = getDocumentTransitionsFixture({
      create: [doc1, doc1],
      replace: [doc2],
    });

    const passingExecutionResult = new DataTriggerExecutionResultJs();
    const executionResultWithErrors = new DataTriggerExecutionResultJs();

    executionResultWithErrors.addError(new Error('Trigger error'));

    const passingTriggerMockFunction = this.sinonSandbox.stub()
      .resolves(passingExecutionResult);
    const throwingTriggerMockFunction = this.sinonSandbox.stub()
      .resolves(executionResultWithErrors);

    const passingDataTriggerMock = new DataTriggerJs(
      contractMockJs.getId(),
      doc1.getType(),
      AbstractDocumentTransition.ACTIONS.CREATE,
      passingTriggerMockFunction,
    );

    const throwingDataTriggerMock = new DataTriggerJs(
      contractMockJs.getId(),
      doc2.getType(),
      AbstractDocumentTransition.ACTIONS.REPLACE,
      throwingTriggerMockFunction,
    );

    getDataTriggersMock
      .withArgs(contractMockJs.getId(), doc1.getType(), AbstractDocumentTransition.ACTIONS.CREATE)
      .returns([passingDataTriggerMock]);

    getDataTriggersMock
      .withArgs(contractMockJs.getId(), doc2.getType(), AbstractDocumentTransition.ACTIONS.REPLACE)
      .returns([throwingDataTriggerMock]);

    contextJs = new DataTriggerExecutionContextJs(
      null, generateRandomIdentifier(), contractMockJs, stateTransitionHeaderMock,
    );

    const dataTriggerExecutionResults = await executeDataTriggersJs(
      documentTransitionsJs, contextJs,
    );

    const expectedResultsCount = 3;

    expect(documentTransitionsJs.length).to.equal(expectedResultsCount);
    expect(dataTriggerExecutionResults.length).to.equal(expectedResultsCount);

    const passingResults = dataTriggerExecutionResults.filter((result) => result.isOk());
    const failingResults = dataTriggerExecutionResults.filter((result) => !result.isOk());

    expect(passingResults).to.have.a.lengthOf(2);
    expect(failingResults).to.have.a.lengthOf(1);

    expect(failingResults[0].getErrors()).to.have.a.lengthOf(1);
    expect(failingResults[0].getErrors()[0].message).to
      .equal('Trigger error');

    expect(passingTriggerMockFunction.callCount).to.equal(2);
    expect(throwingTriggerMockFunction.callCount).to.equal(1);
  });

  it('should return a result for each passed document with success or error - Rust', async function test() {
    const documentTransition = documentTransitions[0];
    const rawDocumentTransition = documentTransition.toObject();
    rawDocumentTransition.normalizedLabel = "a".repeat(257);

    const invalidDocumentCreateTransition = new DocumentCreateTransition(rawDocumentTransition, dataContract.clone());
    const invalidDocumentTransition = DocumentTransition.fromTransitionCreate(invalidDocumentCreateTransition);

    const dataTriggerExecutionResults = await executeDataTriggers(
      [documentTransition, invalidDocumentTransition], context, getAllDataTriggers()
    );

    expect(dataTriggerExecutionResults).to.have.a.lengthOf(2);

    const [validResult, invalidResult] = dataTriggerExecutionResults;
    expect(validResult.isOk()).to.be.true();
    expect(invalidResult.isOk()).to.be.false();
  });

  it("should not call any triggers if documents have no triggers associated with it's type or action", async () => {
    getDataTriggersMock
      .withArgs(
        contractMockJs.getId(),
        domainDocumentType,
        AbstractDocumentTransition.ACTIONS.CREATE,
      )
      .returns([])
      .withArgs(
        contractMockJs.getId(),
        domainDocumentType,
        AbstractDocumentTransition.ACTIONS.DELETE,
      )
      .returns([dpnsDeleteDomainDataTriggerMock])
      .withArgs(
        contractMockJs.getId(),
        domainDocumentType,
        AbstractDocumentTransition.ACTIONS.REPLACE,
      )
      .returns([dpnsUpdateDomainDataTriggerMock]);

    await executeDataTriggersJs(documentTransitionsJs, contextJs);

    expect(dpnsDeleteDomainDataTriggerMock.execute).not.to.be.called();
    expect(dpnsUpdateDomainDataTriggerMock.execute).not.to.be.called();
  });

  it("should not call any triggers if documents have no triggers associated with it's type or action - Rust", async () => {
    const dataTriggers = [getAllDataTriggers()[1]];
    const dataTriggerExecutionResults = await executeDataTriggers(documentTransitions, context, dataTriggers);

    expect(dataTriggerExecutionResults).to.have.a.lengthOf(0);
  });

  it("should call only one trigger if there's one document with a trigger and one without", async () => {
    const dataContractId = getDataContractFixture().getId();
    childDocumentJs.dataContractId = dataContractId;
    childDocumentJs.dataContract.id = dataContractId;
    childDocumentJs.ownerId = IdentifierJs.from(
      getDocumentsFixture.ownerId,
    );

    documentTransitionsJs = getDocumentTransitionsFixture({
      create: [childDocumentJs].concat(getDocumentsFixture(dataContractJs)),
    });

    getDataTriggersMock.resetBehavior();
    getDataTriggersMock
      .returns([])
      .withArgs(
        contractMockJs.getId(),
        domainDocumentType,
        AbstractDocumentTransition.ACTIONS.CREATE,
      )
      .returns([dpnsCreateDomainDataTriggerMock])
      .withArgs(
        contractMockJs.getId(),
        domainDocumentType,
        AbstractDocumentTransition.ACTIONS.DELETE,
      )
      .returns([dpnsDeleteDomainDataTriggerMock])
      .withArgs(
        contractMockJs.getId(),
        domainDocumentType,
        AbstractDocumentTransition.ACTIONS.REPLACE,
      )
      .returns([dpnsUpdateDomainDataTriggerMock]);

    await executeDataTriggersJs(documentTransitionsJs, contextJs);

    expect(dpnsCreateDomainDataTriggerMock.execute).to.be.calledOnce();
    expect(dpnsDeleteDomainDataTriggerMock.execute).not.to.be.called();
    expect(dpnsUpdateDomainDataTriggerMock.execute).not.to.be.called();
  });

  it("should call only one trigger if there's one document with a trigger and one without - Rust", async () => {
    const dataContractId = getDataContractFixture().getId();
    childDocumentJs.dataContractId = dataContractId;
    childDocumentJs.dataContract.id = dataContractId;
    childDocumentJs.ownerId = IdentifierJs.from(
      getDocumentsFixture.ownerId,
    );

    documentTransitionsJs = getDocumentTransitionsFixture({
      create: [childDocumentJs].concat(getDocumentsFixture(dataContractJs)),
    });

    documentTransitions = documentTransitionsJs.map((transition) => {
      const transitionCreate = new DocumentCreateTransition(transition.toObject(), dataContract.clone());
      return DocumentTransition.fromTransitionCreate(transitionCreate);
    });

    const dataTriggerExecutionResults = await executeDataTriggers(documentTransitions, context, getAllDataTriggers());

    expect(dataTriggerExecutionResults).to.have.a.lengthOf(1);
  });

  it("should not call any triggers if there's no triggers in the contract", async () => {
    documentTransitionsJs = getDocumentTransitionsFixture({
      create: getDocumentsFixture(dataContractJs),
    });

    getDataTriggersMock.resetBehavior();
    getDataTriggersMock
      .returns([])
      .withArgs(
        contractMockJs.getId(),
        domainDocumentType,
        AbstractDocumentTransition.ACTIONS.CREATE,
      )
      .returns([dpnsCreateDomainDataTriggerMock])
      .withArgs(
        contractMockJs.getId(),
        domainDocumentType,
        AbstractDocumentTransition.ACTIONS.DELETE,
      )
      .returns([dpnsDeleteDomainDataTriggerMock])
      .withArgs(
        contractMockJs.getId(),
        domainDocumentType,
        AbstractDocumentTransition.ACTIONS.REPLACE,
      )
      .returns([dpnsUpdateDomainDataTriggerMock]);

    await executeDataTriggersJs(documentTransitionsJs, contextJs);

    expect(dpnsCreateDomainDataTriggerMock.execute).not.to.be.called();
    expect(dpnsDeleteDomainDataTriggerMock.execute).not.to.be.called();
    expect(dpnsUpdateDomainDataTriggerMock.execute).not.to.be.called();
  });

  it("should not call any triggers if there's no triggers in the contract - Rust", async () => {
    documentTransitionsJs = getDocumentTransitionsFixture({
      create: getDocumentsFixture(dataContractJs),
    });

    documentTransitions = documentTransitionsJs.map((transition) => {
      const transitionCreate = new DocumentCreateTransition(transition.toObject(), dataContract.clone());
      return DocumentTransition.fromTransitionCreate(transitionCreate);
    });

    const dataTriggerExecutionResults = await executeDataTriggers(documentTransitions, context, getAllDataTriggers());

    expect(dataTriggerExecutionResults).to.have.a.lengthOf(0);
  });
});
