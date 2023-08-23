const dpnsSystemIds = require('@dashevo/dpns-contract/lib/systemIds');
const getDpnsContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDpnsContractFixture');
const dpnsDocumentFixture = require('@dashevo/dpp/lib/test/fixtures/getDpnsDocumentFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentTransitionsFixture');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const IdentifierJs = require('@dashevo/dpp/lib/identifier/Identifier');
const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');

const { default: loadWasmDpp } = require('../../../../../../../dist');

let DataContract;
let DocumentTransition;
let DocumentCreateTransition;
let DataTriggerExecutionContext;
let Document;
let DataTriggerExecutionResult;
let StateTransitionExecutionContext;
let getAllDataTriggers;

describe.skip('executeDataTriggersFactory', () => {
  let childDocumentJs;
  let childDocument;
  let contractMock;
  let stateTransitionExecutionContext;
  let context;
  let documentTransitions;

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

    contractMock = new DataContract(getDpnsContractFixture().toObject());

    childDocumentJs = dpnsDocumentFixture.getChildDocumentFixture();
    childDocument = new Document(
      childDocumentJs.toObject(),
      dataContract.clone(),
      childDocumentJs.getType(),
    );

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    contractMock.setId(dpnsSystemIds.contractId);
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDocuments.resolves([childDocument]);

    stateTransitionExecutionContext = new StateTransitionExecutionContext();
    stateTransitionExecutionContext.disableDryRun();

    context = new DataTriggerExecutionContext(
      stateRepositoryMock,
      childDocument.getOwnerId(),
      contractMock.clone(),
      stateTransitionExecutionContext,
    );

    const documentTransitionsJs = getDocumentTransitionsFixture({
      create: [childDocumentJs],
    });

    documentTransitions = documentTransitionsJs.map((transition) => {
      const transitionCreate = new DocumentCreateTransition(
        transition.toObject(), dataContract.clone(),
      );
      return DocumentTransition.fromTransitionCreate(transitionCreate);
    });
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

  it('should return multiple data triggers if there is more than one data trigger for'
    + ' the same document and action in the contract - Rust', async () => {
    const dataTriggersList = getAllDataTriggers();
    const dataTriggerListWithDuplicates = [
      dataTriggersList[0],
      dataTriggersList[0],
      dataTriggersList[0],
    ];

    const expectedTriggersCount = 3;
    const dataTriggerExecutionResults = await executeDataTriggers(
      documentTransitions, context, dataTriggerListWithDuplicates,
    );

    expect(dataTriggerExecutionResults).to.have.a.lengthOf(expectedTriggersCount);
  });

  it('should return a result for each passed document with success or error - Rust', async () => {
    const documentTransition = documentTransitions[0];
    const rawDocumentTransition = documentTransition.toObject();
    rawDocumentTransition.normalizedLabel = 'a'.repeat(257);

    const invalidDocumentCreateTransition = new DocumentCreateTransition(
      rawDocumentTransition, dataContract.clone(),
    );
    const invalidDocumentTransition = DocumentTransition.fromTransitionCreate(
      invalidDocumentCreateTransition,
    );

    const dataTriggerExecutionResults = await executeDataTriggers(
      [documentTransition, invalidDocumentTransition], context, getAllDataTriggers(),
    );

    expect(dataTriggerExecutionResults).to.have.a.lengthOf(2);

    const [validResult, invalidResult] = dataTriggerExecutionResults;
    expect(validResult.isOk()).to.be.true();
    expect(invalidResult.isOk()).to.be.false();
  });

  it("should not call any triggers if documents have no triggers associated with it's type or action - Rust", async () => {
    const dataTriggers = [getAllDataTriggers()[1]];
    const dataTriggerExecutionResults = await executeDataTriggers(
      documentTransitions, context, dataTriggers,
    );

    expect(dataTriggerExecutionResults).to.have.a.lengthOf(0);
  });

  it("should call only one trigger if there's one document with a trigger and one without - Rust", async () => {
    const dataContractId = getDataContractFixture().getId();
    childDocumentJs.dataContractId = dataContractId;
    childDocumentJs.dataContract.id = dataContractId;
    childDocumentJs.ownerId = IdentifierJs.from(
      getDocumentsFixture.ownerId,
    );

    const documentTransitionsJs = getDocumentTransitionsFixture({
      create: [childDocumentJs].concat(getDocumentsFixture(dataContractJs)),
    });

    documentTransitions = documentTransitionsJs.map((transition) => {
      const transitionCreate = new DocumentCreateTransition(
        transition.toObject(), dataContract.clone(),
      );
      return DocumentTransition.fromTransitionCreate(transitionCreate);
    });

    const dataTriggerExecutionResults = await executeDataTriggers(
      documentTransitions, context, getAllDataTriggers(),
    );

    expect(dataTriggerExecutionResults).to.have.a.lengthOf(1);
  });

  it("should not call any triggers if there's no triggers in the contract - Rust", async () => {
    const documentTransitionsJs = getDocumentTransitionsFixture({
      create: getDocumentsFixture(dataContractJs),
    });

    documentTransitions = documentTransitionsJs.map((transition) => {
      const transitionCreate = new DocumentCreateTransition(
        transition.toObject(), dataContract.clone(),
      );
      return DocumentTransition.fromTransitionCreate(transitionCreate);
    });

    const dataTriggerExecutionResults = await executeDataTriggers(
      documentTransitions, context, getAllDataTriggers(),
    );

    expect(dataTriggerExecutionResults).to.have.a.lengthOf(0);
  });
});
