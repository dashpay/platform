const validateDocumentsBatchTransitionStateFactory = require('@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/validation/state/validateDocumentsBatchTransitionStateFactory');

const DocumentJs = require('@dashevo/dpp/lib/document/Document');
const DocumentsBatchTransitionJs = require('@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/DocumentsBatchTransition');

const DataTriggerExecutionContext = require('@dashevo/dpp/lib/dataTrigger/DataTriggerExecutionContext');
const DataTriggerExecutionError = require('@dashevo/dpp/lib/errors/consensus/state/dataContract/dataTrigger/DataTriggerExecutionError');
const DataTriggerExecutionResult = require('@dashevo/dpp/lib/dataTrigger/DataTriggerExecutionResult');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentTransitionsFixture');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const ValidationResultJs = require('@dashevo/dpp/lib/validation/ValidationResult');

const { expectValidationError } = require('@dashevo/dpp/lib/test/expect/expectError');

const DataContractNotPresentErrorJs = require('@dashevo/dpp/lib/errors/DataContractNotPresentError');

const DocumentAlreadyPresentErrorJs = require('@dashevo/dpp/lib/errors/consensus/state/document/DocumentAlreadyPresentError');
const DocumentNotFoundErrorJs = require('@dashevo/dpp/lib/errors/consensus/state/document/DocumentNotFoundError');
const InvalidDocumentRevisionErrorJs = require('@dashevo/dpp/lib/errors/consensus/state/document/InvalidDocumentRevisionError');
const InvalidDocumentActionErrorJs = require('@dashevo/dpp/lib/document/errors/InvalidDocumentActionError');
const DocumentOwnerIdMismatchErrorJs = require('@dashevo/dpp/lib/errors/consensus/state/document/DocumentOwnerIdMismatchError');
const DocumentTimestampsMismatchErrorJs = require('@dashevo/dpp/lib/errors/consensus/state/document/DocumentTimestampsMismatchError');
const DocumentTimestampWindowViolationErrorJs = require('@dashevo/dpp/lib/errors/consensus/state/document/DocumentTimestampWindowViolationError');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const SomeConsensusError = require('@dashevo/dpp/lib/test/mocks/SomeConsensusError');
const StateTransitionExecutionContextJs = require('@dashevo/dpp/lib/stateTransition/StateTransitionExecutionContext');
const { default: loadWasmDpp } = require('../../../../../../../dist');

let Identifier;
let Document;
let DataContract;
let DocumentsBatchTransition;
let StateTransitionExecutionContext;
let validateDocumentsBatchTransitionState;
let DataContractNotPresentNotConsensusError;
let ValidationResult;

let DocumentNotFoundError;
let InvalidDocumentRevisionError;
let DocumentOwnerIdMismatchError;
let DocumentTimestampsMismatchError;
let DocumentTimestampWindowViolationError;

describe('validateDocumentsBatchTransitionStateFactory', () => {
  let validateDocumentsBatchTransitionStateJs;
  let fetchDocumentsMock;
  let stateTransitionJs;
  let stateTransition;
  let documentsJs;
  let documents;
  let dataContractJs;
  let dataContract;
  let ownerIdJs;
  let validateDocumentsUniquenessByIndicesMock;
  let stateRepositoryMockJs;
  let stateRepositoryMock;
  let executeDataTriggersMock;
  let documentTransitionsJs;
  let fakeTime;
  let blockTime;
  let executionContextJs;
  let executionContext;

  beforeEach(async function beforeEach() {
    ({
      Identifier,
      Document,
      DataContract,
      DocumentsBatchTransition,
      StateTransitionExecutionContext,
      ValidationResult,
      validateDocumentsBatchTransitionState,
      // Errors
      DataContractNotPresentNotConsensusError,

      // New errors
      DocumentAlreadyPresentError,
      DocumentNotFoundError,
      InvalidDocumentRevisionError,
      InvalidDocumentActionError,
      DocumentOwnerIdMismatchError,
      DocumentTimestampsMismatchError,
      DocumentTimestampWindowViolationError,

    } = await loadWasmDpp());

    dataContractJs = getDataContractFixture();
    dataContract = DataContract.fromBuffer(dataContractJs.toBuffer());

    documentsJs = getDocumentsFixture(dataContractJs);
    documents = documentsJs.map((d) => {
      const doc = new Document(d.toObject(), dataContract);
      doc.setEntropy(d.entropy);
      return doc;
    });
    ownerIdJs = getDocumentsFixture.ownerId;

    documentTransitionsJs = getDocumentTransitionsFixture({
      create: documentsJs,
    });

    stateTransitionJs = new DocumentsBatchTransitionJs({
      ownerId: ownerIdJs,
      transitions: documentTransitionsJs.map((t) => t.toObject()),
    }, [dataContractJs]);

    stateTransition = new DocumentsBatchTransition({
      ownerId: ownerIdJs,
      transitions: documentTransitionsJs.map((t) => t.toObject()),
    }, [dataContract.clone()]);

    executionContextJs = new StateTransitionExecutionContextJs();
    executionContext = new StateTransitionExecutionContext();

    stateTransitionJs.setExecutionContext(executionContextJs);
    stateTransition.setExecutionContext(executionContext);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMockJs = createStateRepositoryMock(this.sinonSandbox);

    stateRepositoryMock.fetchDataContract.returns(dataContract.clone());
    stateRepositoryMockJs.fetchDataContract.resolves(dataContractJs);

    blockTime = Date.now();

    stateRepositoryMockJs.fetchLatestPlatformBlockTime.resolves(blockTime);
    stateRepositoryMock.fetchLatestPlatformBlockTime.returns(blockTime);

    fetchDocumentsMock = this.sinonSandbox.stub().resolves([]);
    stateRepositoryMock.fetchDocuments.returns([]);

    executeDataTriggersMock = this.sinonSandbox.stub();
    validateDocumentsUniquenessByIndicesMock = this.sinonSandbox.stub();

    validateDocumentsUniquenessByIndicesMock.resolves(new ValidationResultJs());

    validateDocumentsBatchTransitionStateJs = validateDocumentsBatchTransitionStateFactory(
      stateRepositoryMockJs,
      fetchDocumentsMock,
      validateDocumentsUniquenessByIndicesMock,
      executeDataTriggersMock,
    );


    fakeTime = this.sinonSandbox.useFakeTimers(new Date());
  });

  afterEach(() => {
    fakeTime.reset();
  });

  it('should throw DataContractNotPresentError if data contract was not found', async () => {
    stateRepositoryMockJs.fetchDataContract.resolves(null);

    try {
      await validateDocumentsBatchTransitionStateJs(stateTransitionJs);

      expect.fail('should throw DataContractNotPresentError');
    } catch (e) {
      expect(e).to.be.instanceOf(DataContractNotPresentErrorJs);

      expect(e.getDataContractId()).to.deep.equal(dataContractJs.getId());

      expect(stateRepositoryMockJs.fetchDataContract).to.have.been.calledOnceWithExactly(
        dataContractJs.getId(),
        new StateTransitionExecutionContextJs(),
      );

      expect(fetchDocumentsMock).to.have.not.been.called();
      expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
      expect(executeDataTriggersMock).to.have.not.been.called();
    }
  });

  it('should throw DataContractNotPresentError if data contract was not found - Rust', async () => {
    stateRepositoryMock.fetchDataContract.returns(null);

    try {
      await validateDocumentsBatchTransitionState(stateRepositoryMock, stateTransition);

      expect.fail('should throw DataContractNotPresentError');
    } catch (e) {
      expect(e).to.be.instanceOf(DataContractNotPresentNotConsensusError);

      expect(e.getDataContractId()).to.deep.equal(dataContract.getId().toBuffer());

      expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
      const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
      expect(fetchDataContractId.toBuffer()).to.deep.equal(dataContract.getId().toBuffer());
    }
  });


  it('should return invalid result if document transition with action "create" is already present', async () => {
    fetchDocumentsMock.resolves([documentsJs[0]]);

    const result = await validateDocumentsBatchTransitionStateJs(stateTransitionJs);

    expectValidationError(result, DocumentAlreadyPresentErrorJs);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4004);
    expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
    expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());

    expect(stateRepositoryMockJs.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContractJs.getId(),
      new StateTransitionExecutionContextJs(),
    );

    expect(fetchDocumentsMock.getCall(0).args[0].map((t) => t.toObject())).to.have.deep.members(
      documentTransitionsJs.map((t) => t.toObject()),
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if document transition with action "create" is already present  - Rust', async () => {
    stateRepositoryMock.fetchDocuments.returns([documents[0]]);

    const result = await validateDocumentsBatchTransitionState(stateRepositoryMock, stateTransition);

    expect(result.isValid()).is.not.true();

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4004);
    expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());


    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
    const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
    expect(fetchDataContractId.toBuffer()).to.deep.equal(dataContract.getId().toBuffer());
  });


  it('should return invalid result if document transition with action "replace" is not present', async () => {
    documentTransitionsJs = getDocumentTransitionsFixture({
      create: [],
      replace: [documentsJs[0]],
    });

    stateTransitionJs = new DocumentsBatchTransitionJs({
      ownerId: ownerIdJs,
      contractId: dataContractJs.getId(),
      transitions: documentTransitionsJs.map((t) => t.toObject()),
    }, [dataContractJs]);

    stateTransitionJs.setExecutionContext(executionContextJs);

    const result = await validateDocumentsBatchTransitionStateJs(stateTransitionJs);

    expectValidationError(result, DocumentNotFoundErrorJs);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4005);
    expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
    expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());

    expect(stateRepositoryMockJs.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContractJs.getId(),
      new StateTransitionExecutionContextJs(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      documentTransitionsJs,
      executionContextJs,
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if document transition with action "replace" is not present - Rust', async () => {
    documentTransitionsJs = getDocumentTransitionsFixture({
      create: [],
      replace: [documentsJs[0]],
    });

    stateTransition = new DocumentsBatchTransition({
      ownerId: ownerIdJs,
      contractId: dataContractJs.getId(),
      transitions: documentTransitionsJs.map((t) => t.toObject()),
    }, [dataContract]);


    const result = await validateDocumentsBatchTransitionState(stateRepositoryMock, stateTransition);
    expect(result).is.instanceOf(ValidationResult);

    const [error] = result.getErrors();
    expect(error).is.instanceOf(DocumentNotFoundError);
    expect(error.getCode()).to.equal(4005);
    expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
    const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
    expect(fetchDataContractId.toBuffer()).to.deep.equal(dataContract.getId().toBuffer());

    expect(stateRepositoryMock.fetchDocuments).to.have.been.callCount(documentTransitionsJs.length);
  });

  it('should return invalid result if document transition with action "delete" is not present', async () => {
    documentTransitionsJs = getDocumentTransitionsFixture({
      create: [],
      delete: [documentsJs[0]],
    });

    stateTransitionJs = new DocumentsBatchTransitionJs({
      ownerId: ownerIdJs,
      contractId: dataContractJs.getId(),
      transitions: documentTransitionsJs.map((t) => t.toObject()),
    }, [dataContractJs]);

    stateTransitionJs.setExecutionContext(executionContextJs);

    const result = await validateDocumentsBatchTransitionStateJs(stateTransitionJs);

    expectValidationError(result, DocumentNotFoundErrorJs);
    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4005);
    expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
    expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());

    expect(stateRepositoryMockJs.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContractJs.getId(),
      new StateTransitionExecutionContextJs(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      documentTransitionsJs,
      executionContextJs,
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if document transition with action "delete" is not present - Rust', async () => {
    documentTransitionsJs = getDocumentTransitionsFixture({
      create: [],
      delete: [documentsJs[0]],
    });

    stateTransition = new DocumentsBatchTransition({
      ownerId: ownerIdJs,
      contractId: dataContractJs.getId(),
      transitions: documentTransitionsJs.map((t) => t.toObject()),
    }, [dataContract]);


    const result = await validateDocumentsBatchTransitionState(stateRepositoryMock, stateTransition);

    expect(result).is.instanceOf(ValidationResult);

    const [error] = result.getErrors();
    expect(error).is.instanceOf(DocumentNotFoundError);
    expect(error.getCode()).to.equal(4005);
    expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
    const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
    expect(fetchDataContractId.toBuffer()).to.deep.equal(dataContract.getId().toBuffer());

    expect(stateRepositoryMock.fetchDocuments).to.have.been.callCount(documentTransitionsJs.length);
  });

  it('should return invalid result if document transition with action "replace" has wrong revision', async () => {
    const replaceDocument = new DocumentJs(documentsJs[0].toObject(), dataContractJs);
    replaceDocument.setRevision(3);

    documentTransitionsJs = getDocumentTransitionsFixture({
      create: [],
      replace: [replaceDocument],
    });

    stateTransitionJs = new DocumentsBatchTransitionJs({
      ownerId: ownerIdJs,
      contractId: dataContractJs.getId(),
      transitions: documentTransitionsJs.map((t) => t.toObject()),
    }, [dataContractJs]);

    stateTransitionJs.setExecutionContext(executionContextJs);

    documentsJs[0].setCreatedAt(replaceDocument.getCreatedAt());
    fetchDocumentsMock.resolves([documentsJs[0]]);

    const result = await validateDocumentsBatchTransitionStateJs(stateTransitionJs);

    expectValidationError(result, InvalidDocumentRevisionErrorJs);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4010);
    expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
    expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());
    expect(error.getCurrentRevision()).to.deep.equal(documentsJs[0].getRevision());

    expect(stateRepositoryMockJs.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContractJs.getId(),
      new StateTransitionExecutionContextJs(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      documentTransitionsJs,
      executionContextJs,
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if document transition with action "replace" has wrong revision - Rust', async () => {
    const replaceDocument = new DocumentJs(documentsJs[0].toObject(), dataContractJs);
    replaceDocument.setRevision(3);

    documentTransitionsJs = getDocumentTransitionsFixture({
      create: [],
      replace: [replaceDocument],
    });

    stateTransition = new DocumentsBatchTransition({
      ownerId: ownerIdJs,
      contractId: dataContractJs.getId(),
      transitions: documentTransitionsJs.map((t) => t.toObject()),
    }, [dataContract]);

    documents[0].setCreatedAt(replaceDocument.getCreatedAt());
    stateRepositoryMock.fetchDocuments.returns([documents[0]]);

    const result = await validateDocumentsBatchTransitionState(stateRepositoryMock, stateTransition);

    expect(result).is.instanceOf(ValidationResult);

    const [error] = result.getErrors();
    expect(error).is.instanceOf(InvalidDocumentRevisionError);
    expect(error.getCode()).to.equal(4010);

    expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());
    expect(error.getCurrentRevision()).to.deep.equal(documents[0].getRevision());

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
    const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
    expect(fetchDataContractId.toBuffer()).to.deep.equal(dataContract.getId().toBuffer());

    expect(stateRepositoryMock.fetchDocuments).to.have.been.callCount(documentTransitionsJs.length);
  });

  it('should return invalid result if document transition with action "replace" has mismatch of ownerId with previous revision', async () => {
    const replaceDocument = new DocumentJs(documentsJs[0].toObject(), dataContractJs);
    replaceDocument.setRevision(1);

    const fetchedDocument = new DocumentJs(documentsJs[0].toObject(), dataContractJs);
    fetchedDocument.ownerId = generateRandomIdentifier();

    documentTransitionsJs = getDocumentTransitionsFixture({
      create: [],
      replace: [replaceDocument],
    });

    stateTransitionJs = new DocumentsBatchTransitionJs({
      ownerId: ownerIdJs,
      contractId: dataContractJs.getId(),
      transitions: documentTransitionsJs.map((t) => t.toObject()),
    }, [dataContractJs]);

    stateTransitionJs.setExecutionContext(executionContextJs);

    fetchDocumentsMock.resolves([fetchedDocument]);

    const result = await validateDocumentsBatchTransitionStateJs(stateTransitionJs);

    expectValidationError(result, DocumentOwnerIdMismatchErrorJs);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4006);
    expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
    expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());

    expect(Buffer.isBuffer(error.getDocumentOwnerId())).to.be.true();
    expect(error.getDocumentOwnerId()).to.deep.equal(
      replaceDocument.getOwnerId().toBuffer(),
    );

    expect(Buffer.isBuffer(error.getExistingDocumentOwnerId())).to.be.true();
    expect(error.getExistingDocumentOwnerId()).to.deep.equal(
      fetchedDocument.getOwnerId().toBuffer(),
    );

    expect(stateRepositoryMockJs.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContractJs.getId(),
      new StateTransitionExecutionContextJs(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      documentTransitionsJs,
      executionContextJs,
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if document transition with action "replace" has mismatch of ownerId with previous revision - Rust', async () => {
    const replaceDocument = new DocumentJs(documentsJs[0].toObject(), dataContractJs);
    replaceDocument.setRevision(1);

    const fetchedDocument = new Document(documentsJs[0].toObject(), dataContract);
    fetchedDocument.setOwnerId(Identifier.from(generateRandomIdentifier().toBuffer()));

    documentTransitionsJs = getDocumentTransitionsFixture({
      create: [],
      replace: [replaceDocument],
    });

    stateTransition = new DocumentsBatchTransition({
      ownerId: ownerIdJs,
      contractId: dataContractJs.getId(),
      transitions: documentTransitionsJs.map((t) => t.toObject()),
    }, [dataContract]);

    stateRepositoryMock.fetchDocuments.returns([fetchedDocument]);

    const result = await validateDocumentsBatchTransitionState(stateRepositoryMock, stateTransition);

    expect(result).is.instanceOf(ValidationResult);

    const [error] = result.getErrors();
    expect(error).is.instanceOf(DocumentOwnerIdMismatchError);
    expect(error.getCode()).to.equal(4006);
    expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());
    expect(error.getExistingDocumentOwnerId()).to.deep.equal(fetchedDocument.getOwnerId().toBuffer());

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
    const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
    expect(fetchDataContractId.toBuffer()).to.deep.equal(dataContract.getId().toBuffer());

    expect(stateRepositoryMock.fetchDocuments).to.have.been.callCount(documentTransitionsJs.length);
  });

  it('should throw an error if document transition has invalid action', async () => {
    stateTransitionJs = new DocumentsBatchTransitionJs({
      ownerId: ownerIdJs,
      contractId: dataContractJs.getId(),
      transitions: documentTransitionsJs.map((t) => t.toObject()),
    }, [dataContractJs]);

    stateTransitionJs.setExecutionContext(executionContextJs);

    stateTransitionJs.transitions[0].getAction = () => 5;

    fetchDocumentsMock.resolves([documentsJs[0]]);

    try {
      await validateDocumentsBatchTransitionStateJs(stateTransitionJs);

      expect.fail('InvalidDocumentActionError should be thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidDocumentActionErrorJs);
      expect(e.getDocumentTransition().toObject()).to.deep.equal(
        stateTransitionJs.transitions[0].toObject(),
      );

      expect(stateRepositoryMockJs.fetchDataContract).to.have.been.calledOnceWithExactly(
        dataContractJs.getId(),
        new StateTransitionExecutionContextJs(),
      );

      expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
        stateTransitionJs.transitions,
        executionContextJs,
      );

      expect(validateDocumentsUniquenessByIndicesMock).to.have.not.been.called();
      expect(executeDataTriggersMock).to.have.not.been.called();
    }
  });

  it('should throw an error if document transition has invalid action - Rust', async () => {
    // Omitted - DocumentsBatchTransition cannot be created from the transition with an invalid action
    // because the `DocumentTransition` uses enum
  });

  it('should return invalid result if there are duplicate document transitions according to unique indices', async () => {
    const duplicateDocumentsError = new SomeConsensusError('error');

    validateDocumentsUniquenessByIndicesMock.resolves(
      new ValidationResultJs([duplicateDocumentsError]),
    );

    const result = await validateDocumentsBatchTransitionStateJs(stateTransitionJs);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(duplicateDocumentsError);

    expect(stateRepositoryMockJs.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContractJs.getId(),
      new StateTransitionExecutionContextJs(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      stateTransitionJs.transitions,
      executionContextJs,
    );

    const [callOwnerId, callDocumentTransitions, callDataContract] = (
      validateDocumentsUniquenessByIndicesMock.getCall(0).args
    );

    const callArgs = [
      callOwnerId,
      callDocumentTransitions.map((t) => t.toObject()),
      callDataContract,
    ];

    expect(callArgs).to.have.deep.members([
      ownerIdJs,
      documentTransitionsJs.map((t) => t.toObject()),
      dataContractJs,
    ]);
    expect(executeDataTriggersMock).to.have.not.been.called();
  });

  it('should return invalid result if there are duplicate document transitions according to unique indices - Rust', async () => {
    // Omitted as it seems impossible to generate such a state without having UniqueIndicesValidation mocked

  });

  it('should return invalid result if data triggers execution failed', async () => {
    const dataTriggersExecutionContext = new DataTriggerExecutionContext(
      stateRepositoryMockJs,
      ownerIdJs,
      dataContractJs,
      executionContextJs,
    );

    const dataTriggerExecutionError = new DataTriggerExecutionError(
      documentTransitionsJs[0],
      dataTriggersExecutionContext.getDataContract(),
      dataTriggersExecutionContext.getOwnerId(),
      new Error('error'),
    );

    executeDataTriggersMock.resolves([
      new DataTriggerExecutionResult([dataTriggerExecutionError]),
    ]);

    const result = await validateDocumentsBatchTransitionStateJs(stateTransitionJs);

    expectValidationError(result, DataTriggerExecutionError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4002);
    expect(error).to.equal(dataTriggerExecutionError);

    expect(stateRepositoryMockJs.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContractJs.getId(),
      new StateTransitionExecutionContextJs(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      stateTransitionJs.transitions,
      executionContextJs,
    );

    const [callOwnerId, callDocumentTransitions, callDataContract, callExecutionContext] = (
      validateDocumentsUniquenessByIndicesMock.getCall(0).args
    );

    const callArgs = [
      callOwnerId,
      callDocumentTransitions.map((t) => t.toObject()),
      callDataContract,
      callExecutionContext,
    ];

    expect(callArgs).to.have.deep.members([
      ownerIdJs,
      documentTransitionsJs.map((t) => t.toObject()),
      dataContractJs,
      executionContextJs,
    ]);

    const [triggerCallDocumentTransitions, triggerCallDataTriggersExecutionContext] = (
      executeDataTriggersMock.getCall(0).args
    );

    const triggerCallArgs = [
      triggerCallDocumentTransitions.map((t) => t.toObject()),
      triggerCallDataTriggersExecutionContext,
    ];

    expect(triggerCallArgs).to.have.deep.members([
      documentTransitionsJs.map((t) => t.toObject()),
      dataTriggersExecutionContext,
    ]);
  });

  it('should return invalid result if data triggers execution failed', async () => {
    // Omitted as it seems impossible to generate such a state without having DataTrigger execution mocked
  });

  describe('Timestamps', () => {
    let timeWindowStart;
    let timeWindowEnd;

    beforeEach(() => {
      timeWindowStart = new Date(blockTime);
      timeWindowStart.setMinutes(
        timeWindowStart.getMinutes() - 5,
      );

      timeWindowEnd = new Date(blockTime);
      timeWindowEnd.setMinutes(
        timeWindowEnd.getMinutes() + 5,
      );
    });

    describe('CREATE transition', () => {
      it('should return invalid result if timestamps mismatch', async () => {
        documentTransitionsJs = getDocumentTransitionsFixture({
          create: [documentsJs[0]],
        });

        stateTransitionJs = new DocumentsBatchTransitionJs({
          ownerId: ownerIdJs,
          contractId: dataContractJs.getId(),
          transitions: documentTransitionsJs.map((t) => t.toObject()),
        }, [dataContractJs]);

        stateTransitionJs.transitions.forEach((t) => {
          // eslint-disable-next-line no-param-reassign
          t.updatedAt = new Date();
        });

        const result = await validateDocumentsBatchTransitionStateJs(stateTransitionJs);

        expectValidationError(result, DocumentTimestampsMismatchErrorJs);

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(4007);

        documentTransitionsJs[0].updatedAt = new Date();

        expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
        expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());
      });

      it('should return invalid result if timestamps mismatch - Rust', async () => {
        documentTransitionsJs = getDocumentTransitionsFixture({
          create: [documentsJs[0]],
        });

        stateTransition = new DocumentsBatchTransition({
          ownerId: ownerIdJs,
          contractId: dataContractJs.getId(),
          transitions: documentTransitionsJs.map((t) => t.toObject()),
        }, [dataContract]);


        const transitions = stateTransition.getTransitions();
        transitions.forEach((t) => {
          // eslint-disable-next-line no-param-reassign
          t.setUpdatedAt(new Date().getMilliseconds());
        });
        stateTransition.setTransitions(transitions);

        const result = await validateDocumentsBatchTransitionState(stateRepositoryMock, stateTransition);

        expect(result).is.instanceOf(ValidationResult);

        const [error] = result.getErrors();
        expect(error).is.instanceOf(DocumentTimestampsMismatchError);
        expect(error.getCode()).to.equal(4007);
      });

      it('should return invalid result if "$createdAt" have violated time window', async () => {
        documentTransitionsJs = getDocumentTransitionsFixture({
          create: [documentsJs[0]],
        });

        stateTransitionJs = new DocumentsBatchTransitionJs({
          ownerId: ownerIdJs,
          contractId: dataContractJs.getId(),
          transitions: documentTransitionsJs.map((t) => t.toObject()),
        }, [dataContractJs]);

        stateTransitionJs.transitions.forEach((t) => {
          // eslint-disable-next-line no-param-reassign
          t.createdAt.setMinutes(t.createdAt.getMinutes() - 6);
          // eslint-disable-next-line no-param-reassign
          t.updatedAt = undefined;
        });

        const result = await validateDocumentsBatchTransitionStateJs(stateTransitionJs);

        expectValidationError(result, DocumentTimestampWindowViolationErrorJs);

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(4008);

        documentTransitionsJs[0].createdAt.setMinutes(
          documentTransitionsJs[0].createdAt.getMinutes() - 6,
        );
        documentTransitionsJs[0].updatedAt = undefined;

        expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
        expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());
        expect(error.getTimestampName()).to.equal('createdAt');
        expect(error.getTimestamp()).to.deep.equal(documentTransitionsJs[0].createdAt);
        expect(error.getTimeWindowStart()).to.deep.equal(timeWindowStart);
        expect(error.getTimeWindowEnd()).to.deep.equal(timeWindowEnd);
      });

      it('should return invalid result if "$createdAt" have violated time window  - Rust', async () => {
        documentTransitionsJs = getDocumentTransitionsFixture({
          create: [documentsJs[0]],
        });

        stateTransition = new DocumentsBatchTransition({
          ownerId: ownerIdJs,
          contractId: dataContractJs.getId(),
          transitions: documentTransitionsJs.map((t) => t.toObject()),
        }, [dataContract]);

        const transitions = stateTransition.getTransitions();
        transitions.forEach((t) => {
          const createdAtMinus6Mins = t.getCreatedAt() - BigInt(6 * 60 * 1000);
          t.setCreatedAt(createdAtMinus6Mins);
          t.setUpdatedAt(undefined);
        });
        stateTransition.setTransitions(transitions);

        const result = await validateDocumentsBatchTransitionState(stateRepositoryMock, stateTransition);

        expect(result).is.instanceOf(ValidationResult);
        expect(result.isValid()).is.not.true();

        const [error] = result.getErrors();
        expect(error).is.instanceOf(DocumentTimestampWindowViolationError);
        expect(error.getCode()).to.equal(4008);
        expect(error.getTimestampName()).to.equal('createdAt');


        expect(error.getTimestamp().getMilliseconds()).to.equal(documentTransitionsJs[0].createdAt.getMilliseconds());
        expect(error.getTimeWindowStart()).to.deep.equal(timeWindowStart);
        expect(error.getTimeWindowEnd()).to.deep.equal(timeWindowEnd);
      });

      it('should return invalid result if "$updatedAt" have violated time window', async () => {
        documentTransitionsJs = getDocumentTransitionsFixture({
          create: [documentsJs[1]],
        });

        stateTransitionJs = new DocumentsBatchTransitionJs({
          ownerId: ownerIdJs,
          contractId: dataContractJs.getId(),
          transitions: documentTransitionsJs.map((t) => t.toObject()),
        }, [dataContractJs]);

        stateTransitionJs.transitions.forEach((t) => {
          // eslint-disable-next-line no-param-reassign
          t.updatedAt.setMinutes(t.updatedAt.getMinutes() - 6);
          // eslint-disable-next-line no-param-reassign
          t.createdAt = undefined;
        });

        const result = await validateDocumentsBatchTransitionStateJs(stateTransitionJs);

        expectValidationError(result, DocumentTimestampWindowViolationErrorJs);

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(4008);

        documentTransitionsJs[0].updatedAt.setMinutes(
          documentTransitionsJs[0].updatedAt.getMinutes() - 6,
        );
        documentTransitionsJs[0].createdAt = undefined;

        expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
        expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());
        expect(error.getTimestampName()).to.equal('updatedAt');
        expect(error.getTimestamp()).to.deep.equal(documentTransitionsJs[0].updatedAt);
        expect(error.getTimeWindowStart()).to.deep.equal(timeWindowStart);
        expect(error.getTimeWindowEnd()).to.deep.equal(timeWindowEnd);
      });

      it('should return invalid result if "$updatedAt" have violated time window - Rust', async () => {
        documentTransitionsJs = getDocumentTransitionsFixture({
          create: [documentsJs[1]],
        });

        stateTransition = new DocumentsBatchTransition({
          ownerId: ownerIdJs,
          contractId: dataContractJs.getId(),
          transitions: documentTransitionsJs.map((t) => t.toObject()),
        }, [dataContract]);


        const transitions = stateTransition.getTransitions();
        transitions.forEach((t) => {
          const createdAtMinus6Mins = t.getUpdatedAt() - BigInt(6 * 60 * 1000);
          t.setUpdatedAt(createdAtMinus6Mins);
          t.setCreatedAt(undefined);
        });
        stateTransition.setTransitions(transitions);

        const result = await validateDocumentsBatchTransitionState(stateRepositoryMock, stateTransition);

        expect(result).is.instanceOf(ValidationResult);
        expect(result.isValid()).is.not.true();

        const [error] = result.getErrors();
        expect(error).is.instanceOf(DocumentTimestampWindowViolationError);
        expect(error.getCode()).to.equal(4008);

        documentTransitionsJs[0].updatedAt.setMinutes(
          documentTransitionsJs[0].updatedAt.getMinutes() - 6,
        );
        documentTransitionsJs[0].createdAt = undefined;

        expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());
        expect(error.getTimestampName()).to.equal('updatedAt');
        expect(error.getTimestamp().getMilliseconds()).to.deep.equal(documentTransitionsJs[0].updatedAt.getMilliseconds());
        expect(error.getTimeWindowStart()).to.deep.equal(timeWindowStart);
        expect(error.getTimeWindowEnd()).to.deep.equal(timeWindowEnd);
      });

      it('should not validate time in block window on dry run', async () => {
        documentTransitionsJs = getDocumentTransitionsFixture({
          create: [documentsJs[1]],
        });

        executeDataTriggersMock.resolves([
          new DataTriggerExecutionResult(),
        ]);

        stateTransitionJs = new DocumentsBatchTransitionJs({
          ownerId: ownerIdJs,
          contractId: dataContractJs.getId(),
          transitions: documentTransitionsJs.map((t) => t.toObject()),
        }, [dataContractJs]);

        stateTransitionJs.transitions.forEach((t) => {
          // eslint-disable-next-line no-param-reassign
          t.updatedAt.setMinutes(t.updatedAt.getMinutes() - 6);
        });

        stateTransitionJs.getExecutionContext().enableDryRun();

        const result = await validateDocumentsBatchTransitionStateJs(stateTransitionJs);

        stateTransitionJs.getExecutionContext().disableDryRun();

        expect(result).to.be.an.instanceOf(ValidationResultJs);
        expect(result.isValid()).to.be.true();
      });

      it('should not validate time in block window on dry run - Rust', async () => {
        documentTransitionsJs = getDocumentTransitionsFixture({
          create: [documentsJs[1]],
        });

        executeDataTriggersMock.resolves([
          new DataTriggerExecutionResult(),
        ]);

        stateTransition = new DocumentsBatchTransition({
          ownerId: ownerIdJs,
          contractId: dataContractJs.getId(),
          transitions: documentTransitionsJs.map((t) => t.toObject()),
        }, [dataContract]);

        const transitions = stateTransition.getTransitions();
        transitions.forEach((t) => {
          const createdAtMinus6Mins = t.getUpdatedAt() - BigInt(6 * 60 * 1000);
          t.setUpdatedAt(createdAtMinus6Mins);
          t.setCreatedAt(undefined);
        });
        stateTransition.setTransitions(transitions);
        stateTransition.getExecutionContext().enableDryRun();

        const result = await validateDocumentsBatchTransitionState(stateRepositoryMock, stateTransition);

        expect(result).to.be.an.instanceOf(ValidationResult);
        expect(result.isValid()).to.be.true();

      });

      it('should return valid result if timestamps mismatch on dry run', async () => {
        documentTransitionsJs = getDocumentTransitionsFixture({
          create: [documentsJs[0]],
        });

        executeDataTriggersMock.resolves([
          new DataTriggerExecutionResult(),
        ]);

        stateTransitionJs = new DocumentsBatchTransitionJs({
          ownerId: ownerIdJs,
          contractId: dataContractJs.getId(),
          transitions: documentTransitionsJs.map((t) => t.toObject()),
        }, [dataContractJs]);

        stateTransitionJs.transitions.forEach((t) => {
          // eslint-disable-next-line no-param-reassign
          t.updatedAt = new Date();
        });

        stateTransitionJs.getExecutionContext().enableDryRun();

        const result = await validateDocumentsBatchTransitionStateJs(stateTransitionJs);

        stateTransitionJs.getExecutionContext().disableDryRun();

        expect(result).to.be.an.instanceOf(ValidationResultJs);
        expect(result.isValid()).to.be.true();
      });

      it('should return valid result if timestamps mismatch on dry run', async () => {
        documentTransitionsJs = getDocumentTransitionsFixture({
          create: [documentsJs[0]],
        });

        executeDataTriggersMock.resolves([
          new DataTriggerExecutionResult(),
        ]);

        stateTransition = new DocumentsBatchTransition({
          ownerId: ownerIdJs,
          contractId: dataContractJs.getId(),
          transitions: documentTransitionsJs.map((t) => t.toObject()),
        }, [dataContract]);

        const transitions = stateTransition.getTransitions();
        transitions.forEach((t) => {
          // eslint-disable-next-line no-param-reassign
          t.setUpdatedAt(new Date().getMilliseconds());
        });
        stateTransition.getExecutionContext().enableDryRun();

        const result = await validateDocumentsBatchTransitionState(stateRepositoryMock, stateTransition);

        expect(result).to.be.an.instanceOf(ValidationResult);
        expect(result.isValid()).to.be.true();
      });
    });

    describe('REPLACE transition', () => {
      it('should return invalid result if documents with action "replace" have violated time window', async () => {
        documentTransitionsJs = getDocumentTransitionsFixture({
          create: [],
          replace: [documentsJs[1]],
        });

        stateTransitionJs = new DocumentsBatchTransitionJs({
          ownerId: ownerIdJs,
          contractId: dataContractJs.getId(),
          transitions: documentTransitionsJs.map((t) => t.toObject()),
        }, [dataContractJs]);

        documentsJs[1].updatedAt.setMinutes(
          documentsJs[1].updatedAt.getMinutes() - 6,
        );

        fetchDocumentsMock.resolves([documentsJs[1]]);

        stateTransitionJs.transitions.forEach((t) => {
          // eslint-disable-next-line no-param-reassign
          t.updatedAt.setMinutes(t.updatedAt.getMinutes() - 6);
        });

        const result = await validateDocumentsBatchTransitionStateJs(stateTransitionJs);

        expectValidationError(result, DocumentTimestampWindowViolationErrorJs);

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(4008);

        documentTransitionsJs[0].updatedAt.setMinutes(
          documentTransitionsJs[0].updatedAt.getMinutes() - 6,
        );

        expect(Buffer.isBuffer(error.getDocumentId())).to.be.true();
        expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());
        expect(error.getTimestampName()).to.equal('updatedAt');
        expect(error.getTimestamp()).to.deep.equal(documentTransitionsJs[0].updatedAt);
        expect(error.getTimeWindowStart()).to.deep.equal(timeWindowStart);
        expect(error.getTimeWindowEnd()).to.deep.equal(timeWindowEnd);
      });

      it('should return invalid result if documents with action "replace" have violated time window - Rust', async () => {
        documentTransitionsJs = getDocumentTransitionsFixture({
          create: [],
          replace: [documentsJs[1]],
        });

        stateTransition = new DocumentsBatchTransition({
          ownerId: ownerIdJs,
          contractId: dataContractJs.getId(),
          transitions: documentTransitionsJs.map((t) => t.toObject()),
        }, [dataContract]);

        documentsJs[1].updatedAt.setMinutes(
          documentsJs[1].updatedAt.getMinutes() - 6,
        );

        const documentToReturn = new Document(documentsJs[1].toObject(), dataContract);
        stateRepositoryMock.fetchDocuments.returns([documentToReturn])

        const transitions = stateTransition.getTransitions();
        transitions.forEach((t) => {
          const createdAtMinus6Mins = t.getUpdatedAt() - BigInt(6 * 60 * 1000);
          t.setUpdatedAt(createdAtMinus6Mins);
        });
        stateTransition.setTransitions(transitions);

        const result = await validateDocumentsBatchTransitionState(stateRepositoryMock, stateTransition);

        expect(result).is.instanceOf(ValidationResult);
        expect(result.isValid()).is.not.true();
        const [error] = result.getErrors();

        expect(error).is.instanceOf(DocumentTimestampWindowViolationError);
        expect(error.getCode()).to.equal(4008);

        documentTransitionsJs[0].updatedAt.setMinutes(
          documentTransitionsJs[0].updatedAt.getMinutes() - 6,
        );

        expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());
        expect(error.getTimestampName()).to.equal('updatedAt');
        expect(error.getTimestamp()).to.deep.equal(documentTransitionsJs[0].updatedAt);
        expect(error.getTimeWindowStart()).to.deep.equal(timeWindowStart);
        expect(error.getTimeWindowEnd()).to.deep.equal(timeWindowEnd);
      });

      it('should return valid result if documents with action "replace" have violated time window on dry run', async () => {
        executeDataTriggersMock.resolves([
          new DataTriggerExecutionResult(),
        ]);

        documentTransitionsJs = getDocumentTransitionsFixture({
          create: [],
          replace: [documentsJs[1]],
        });

        stateTransitionJs = new DocumentsBatchTransitionJs({
          ownerId: ownerIdJs,
          contractId: dataContractJs.getId(),
          transitions: documentTransitionsJs.map((t) => t.toObject()),
        }, [dataContractJs]);

        documentsJs[1].updatedAt.setMinutes(
          documentsJs[1].updatedAt.getMinutes() - 6,
        );

        fetchDocumentsMock.resolves([documentsJs[1]]);

        stateTransitionJs.transitions.forEach((t) => {
          // eslint-disable-next-line no-param-reassign
          t.updatedAt.setMinutes(t.updatedAt.getMinutes() - 6);
        });

        stateTransitionJs.getExecutionContext().enableDryRun();

        const result = await validateDocumentsBatchTransitionStateJs(stateTransitionJs);

        stateTransitionJs.getExecutionContext().disableDryRun();

        expect(result).to.be.an.instanceOf(ValidationResultJs);
        expect(result.isValid()).to.be.true();
      });

      it('should return valid result if documents with action "replace" have violated time window on dry run - Rust', async () => {
        documentTransitionsJs = getDocumentTransitionsFixture({
          create: [],
          replace: [documentsJs[1]],
        });

        stateTransition = new DocumentsBatchTransition({
          ownerId: ownerIdJs,
          contractId: dataContractJs.getId(),
          transitions: documentTransitionsJs.map((t) => t.toObject()),
        }, [dataContract]);

        documentsJs[1].updatedAt.setMinutes(
          documentsJs[1].updatedAt.getMinutes() - 6,
        );

        const documentToReturn = new Document(documentsJs[1].toObject(), dataContract);
        stateRepositoryMock.fetchDocuments.returns([documentToReturn])

        const transitions = stateTransition.getTransitions(); transitions.forEach((t) => {
          const createdAtMinus6Mins = t.getUpdatedAt() - BigInt(6 * 60 * 1000);
          t.setUpdatedAt(createdAtMinus6Mins);
        });
        stateTransition.setTransitions(transitions);
        stateTransition.getExecutionContext().enableDryRun();

        const result = await validateDocumentsBatchTransitionState(stateRepositoryMock, stateTransition);

        expect(result).to.be.an.instanceOf(ValidationResult);
        expect(result.isValid()).to.be.true();
      });
    });
  });

  it('should return valid result if document transitions are valid', async () => {
    const fetchedDocuments = [
      new DocumentJs(documentsJs[1].toObject(), dataContractJs),
      new DocumentJs(documentsJs[2].toObject(), dataContractJs),
    ];

    fetchDocumentsMock.resolves(fetchedDocuments);

    documentsJs[1].setRevision(1);
    documentsJs[2].setRevision(1);

    documentTransitionsJs = getDocumentTransitionsFixture({
      create: [],
      replace: [documentsJs[1]],
      delete: [documentsJs[2]],
    });

    stateTransitionJs = new DocumentsBatchTransitionJs({
      ownerId: ownerIdJs,
      contractId: dataContractJs.getId(),
      transitions: documentTransitionsJs.map((t) => t.toObject()),
    }, [dataContractJs]);

    stateTransitionJs.setExecutionContext(executionContextJs);

    const dataTriggersExecutionContext = new DataTriggerExecutionContext(
      stateRepositoryMockJs,
      ownerIdJs,
      dataContractJs,
      executionContextJs,
    );

    executeDataTriggersMock.resolves([
      new DataTriggerExecutionResult(),
    ]);

    const result = await validateDocumentsBatchTransitionStateJs(stateTransitionJs);

    expect(result).to.be.an.instanceOf(ValidationResultJs);
    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMockJs.fetchDataContract).to.have.been.calledOnceWithExactly(
      dataContractJs.getId(),
      new StateTransitionExecutionContextJs(),
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      stateTransitionJs.transitions,
      executionContextJs,
    );

    expect(validateDocumentsUniquenessByIndicesMock).to.have.been.calledOnceWithExactly(
      ownerIdJs,
      [documentTransitionsJs[0]],
      dataContractJs,
      executionContextJs,
    );

    expect(executeDataTriggersMock).to.have.been.calledOnceWithExactly(
      documentTransitionsJs,
      dataTriggersExecutionContext,
    );
  });

  it('should return valid result if document transitions are valid - Rust', async () => {
    const fetchedDocuments = [
      new Document(documentsJs[1].toObject(), dataContract),
      new Document(documentsJs[2].toObject(), dataContract),
    ];

    stateRepositoryMock.fetchDocuments.returns(fetchedDocuments);

    documentsJs[1].setRevision(1);
    documentsJs[2].setRevision(1);

    documentTransitionsJs = getDocumentTransitionsFixture({
      create: [],
      replace: [documentsJs[1]],
      delete: [documentsJs[2]],
    });

    stateTransition = new DocumentsBatchTransition({
      ownerId: ownerIdJs,
      contractId: dataContractJs.getId(),
      transitions: documentTransitionsJs.map((t) => t.toObject()),
    }, [dataContract]);

    const result = await validateDocumentsBatchTransitionState(stateRepositoryMock, stateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
    const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
    expect(fetchDataContractId.toBuffer()).to.deep.equal(dataContract.getId().toBuffer());

    expect(stateRepositoryMock.fetchDocuments).to.have.been.calledOnce();
  });
});
