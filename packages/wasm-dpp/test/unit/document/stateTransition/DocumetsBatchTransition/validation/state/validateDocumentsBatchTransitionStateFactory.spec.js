/* eslint-disable */
// TODO: fix this test?
const generateRandomIdentifier = require('../../../../../../../lib/test/utils/generateRandomIdentifierAsync');
const getDataContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../../../../../lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('../../../../../../../lib/test/fixtures/getDocumentTransitionsFixture');
const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');

const { default: loadWasmDpp } = require('../../../../../../../dist');

let Identifier;
let ExtendedDocument;
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

describe.skip('validateDocumentsBatchTransitionStateFactory', () => {
  let stateTransitionJs;
  let stateTransition;
  let documentsJs;
  let documents;
  let dataContractJs;
  let dataContract;
  let ownerId;
  let validateDocumentsUniquenessByIndicesMock;
  let stateRepositoryMockJs;
  let stateRepositoryMock;
  let executeDataTriggersMock;
  let documentTransitions;
  let fakeTime;
  let blockTime;
  let executionContext;

  beforeEach(async function beforeEach() {
    ({
      Identifier,
      ExtendedDocument,
      DataContract,
      DocumentsBatchTransition,
      StateTransitionExecutionContext,
      ValidationResult,
      validateDocumentsBatchTransitionState,
      // Errors
      DataContractNotPresentNotConsensusError,
      DocumentNotFoundError,
      InvalidDocumentRevisionError,
      DocumentOwnerIdMismatchError,
      DocumentTimestampsMismatchError,
      DocumentTimestampWindowViolationError,
    } = await loadWasmDpp());

    dataContract = await getDataContractFixture();
    documents = await getDocumentsFixture(dataContract);
    ownerId = getDocumentsFixture.ownerId;

    documentTransitions = await getDocumentTransitionsFixture({
      create: documents,
    });

    stateTransition = new DocumentsBatchTransition();
    // TODO: add method
    // stateTransition.setOwnerId(ownerId)
    stateTransition.setTransitions(documentTransitions);

    executionContext = new StateTransitionExecutionContext();

    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    stateRepositoryMock.fetchDataContract.resolves(dataContract.clone());

    blockTime = Date.now();

    stateRepositoryMockJs.fetchLatestPlatformBlockTime.resolves(blockTime);
    stateRepositoryMock.fetchLatestPlatformBlockTime.resolves(blockTime);

    stateRepositoryMock.fetchExtendedDocuments.resolves([]);
    stateRepositoryMock.fetchDocuments.resolves([]);

    executeDataTriggersMock = this.sinon.stub();
    validateDocumentsUniquenessByIndicesMock = this.sinon.stub();

    validateDocumentsUniquenessByIndicesMock.resolves(new ValidationResultJs());

    fakeTime = this.sinon.useFakeTimers(new Date());
  });

  afterEach(() => {
    fakeTime.reset();
  });

  // it('should throw DataContractNotPresentError if data contract was not found - Rust', async () => {
  //   stateRepositoryMock.fetchDataContract.resolves(null);
  //
  //   try {
  //     await validateDocumentsBatchTransitionState(
  //       stateRepositoryMock, stateTransition, executionContext,
  //     );
  //
  //     expect.fail('should throw DataContractNotPresentError');
  //   } catch (e) {
  //     expect(e).to.be.instanceOf(DataContractNotPresentNotConsensusError);
  //
  //     expect(e.getDataContractId()).to.deep.equal(dataContract.getId().toBuffer());
  //
  //     expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
  //     const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
  //     expect(fetchDataContractId.toBuffer()).to.deep.equal(dataContract.getId().toBuffer());
  //   }
  // });
  //
  // it('should return invalid result if document transition with action "create" is already present  - Rust', async () => {
  //   stateRepositoryMock.fetchDocuments.resolves([documents[0].getDocument()]);
  //
  //   const result = await validateDocumentsBatchTransitionState(
  //     stateRepositoryMock, stateTransition, executionContext,
  //   );
  //
  //   expect(result.isValid()).is.not.true();
  //
  //   const [error] = result.getErrors();
  //
  //   expect(error.getCode()).to.equal(4004);
  //   expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());
  //
  //   expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
  //   const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
  //   expect(fetchDataContractId.toBuffer()).to.deep.equal(dataContract.getId().toBuffer());
  // });
  //
  // it('should return invalid result if document transition with action "replace" is not present - Rust', async () => {
  //   documentTransitionsJs = getDocumentTransitionsFixture({
  //     create: [],
  //     replace: [documentsJs[0]],
  //   });
  //
  //   stateTransition = new DocumentsBatchTransition({
  //     ownerId: ownerIdJs,
  //     contractId: dataContractJs.getId(),
  //     transitions: documentTransitionsJs.map((t) => t.toObject()),
  //   }, [dataContract]);
  //
  //   const result = await validateDocumentsBatchTransitionState(
  //     stateRepositoryMock, stateTransition, executionContext,
  //   );
  //   expect(result).is.instanceOf(ValidationResult);
  //
  //   const [error] = result.getErrors();
  //   expect(error).is.instanceOf(DocumentNotFoundError);
  //   expect(error.getCode()).to.equal(4005);
  //   expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());
  //
  //   expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
  //   const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
  //   expect(fetchDataContractId.toBuffer()).to.deep.equal(dataContract.getId().toBuffer());
  //
  //   expect(stateRepositoryMock.fetchDocuments)
  //     .to.have.been.callCount(documentTransitionsJs.length);
  // });
  //
  // it('should return invalid result if document transition with action "delete" is not present - Rust', async () => {
  //   documentTransitionsJs = getDocumentTransitionsFixture({
  //     create: [],
  //     delete: [documentsJs[0]],
  //   });
  //
  //   stateTransition = new DocumentsBatchTransition({
  //     ownerId: ownerIdJs,
  //     contractId: dataContractJs.getId(),
  //     transitions: documentTransitionsJs.map((t) => t.toObject()),
  //   }, [dataContract]);
  //
  //   const result = await validateDocumentsBatchTransitionState(
  //     stateRepositoryMock, stateTransition, executionContext,
  //   );
  //
  //   expect(result).is.instanceOf(ValidationResult);
  //
  //   const [error] = result.getErrors();
  //   expect(error).is.instanceOf(DocumentNotFoundError);
  //   expect(error.getCode()).to.equal(4005);
  //   expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());
  //
  //   expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
  //   const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
  //   expect(fetchDataContractId.toBuffer()).to.deep.equal(dataContract.getId().toBuffer());
  //
  //   expect(stateRepositoryMock.fetchDocuments).to.have.been
  //     .callCount(documentTransitionsJs.length);
  // });
  //
  // it('should return invalid result if document transition with action "replace" has wrong revision - Rust', async () => {
  //   const replaceDocument = new DocumentJs(documentsJs[0].toObject(), dataContractJs);
  //   replaceDocument.setRevision(3);
  //
  //   documentTransitionsJs = getDocumentTransitionsFixture({
  //     create: [],
  //     replace: [replaceDocument],
  //   });
  //
  //   stateTransition = new DocumentsBatchTransition({
  //     ownerId: ownerIdJs,
  //     contractId: dataContractJs.getId(),
  //     transitions: documentTransitionsJs.map((t) => t.toObject()),
  //   }, [dataContract]);
  //
  //   documents[0].setCreatedAt(replaceDocument.getCreatedAt());
  //   stateRepositoryMock.fetchDocuments.resolves([documents[0].getDocument()]);
  //
  //   const result = await validateDocumentsBatchTransitionState(
  //     stateRepositoryMock, stateTransition, executionContext,
  //   );
  //
  //   expect(result).is.instanceOf(ValidationResult);
  //
  //   const [error] = result.getErrors();
  //   expect(error).is.instanceOf(InvalidDocumentRevisionError);
  //   expect(error.getCode()).to.equal(4010);
  //
  //   expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());
  //   expect(Number(error.getCurrentRevision())).to.deep.equal(documents[0].getRevision());
  //
  //   expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
  //   const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
  //   expect(fetchDataContractId.toBuffer()).to.deep.equal(dataContract.getId().toBuffer());
  //
  //   expect(stateRepositoryMock.fetchDocuments).to.have.been
  //     .callCount(documentTransitionsJs.length);
  // });
  //
  // it('should return invalid result if document transition with action "replace" has mismatch of ownerId with previous revision - Rust', async () => {
  //   const replaceDocument = new DocumentJs(documentsJs[0].toObject(), dataContractJs);
  //   replaceDocument.setRevision(1);
  //
  //   const fetchedDocument = new ExtendedDocument(documentsJs[0].toObject(), dataContract);
  //   fetchedDocument.setOwnerId(Identifier.from((await generateRandomIdentifier()).toBuffer()));
  //
  //   documentTransitionsJs = getDocumentTransitionsFixture({
  //     create: [],
  //     replace: [replaceDocument],
  //   });
  //
  //   stateTransition = new DocumentsBatchTransition({
  //     ownerId: ownerIdJs,
  //     contractId: dataContractJs.getId(),
  //     transitions: documentTransitionsJs.map((t) => t.toObject()),
  //   }, [dataContract]);
  //
  //   stateRepositoryMock.fetchDocuments.resolves([fetchedDocument.getDocument()]);
  //
  //   const result = await validateDocumentsBatchTransitionState(
  //     stateRepositoryMock,
  //     stateTransition,
  //     executionContext,
  //   );
  //
  //   expect(result).is.instanceOf(ValidationResult);
  //
  //   const [error] = result.getErrors();
  //   expect(error).is.instanceOf(DocumentOwnerIdMismatchError);
  //   expect(error.getCode()).to.equal(4006);
  //   expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());
  //   expect(error.getExistingDocumentOwnerId()).to.deep.equal(
  //     fetchedDocument.getOwnerId().toBuffer(),
  //   );
  //
  //   expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
  //   const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
  //   expect(fetchDataContractId.toBuffer()).to.deep.equal(dataContract.getId().toBuffer());
  //
  //   expect(stateRepositoryMock.fetchDocuments).to.have.been
  //     .callCount(documentTransitionsJs.length);
  // });
  //
  // it('should throw an error if document transition has invalid action - Rust', async () => {
  //   // Omitted - DocumentsBatchTransition cannot be created from the
  //   // transition with an invalid action because the `DocumentTransition`
  //   // uses enums
  // });
  //
  // it('should return invalid result if there are duplicate document transitions according to unique indices - Rust', async () => {
  //   // Omitted as it seems impossible to generate such a state without
  //   // having UniqueIndicesValidation mocked
  // });
  //
  // it('should return invalid result if data triggers execution failed - Rust', async () => {
  //   // Omitted as it seems impossible to generate such a state without
  //   // having DataTrigger execution mocked
  // });
  //
  // it('should return invalid result if data triggers execution failed', async () => {
  //   // Omitted as it seems impossible to generate such a state without
  //   // having DataTrigger execution mocked
  // });
  //
  // describe('Timestamps', () => {
  //   let timeWindowStart;
  //   let timeWindowEnd;
  //
  //   beforeEach(() => {
  //     timeWindowStart = new Date(blockTime);
  //     timeWindowStart.setMinutes(
  //       timeWindowStart.getMinutes() - 5,
  //     );
  //
  //     timeWindowEnd = new Date(blockTime);
  //     timeWindowEnd.setMinutes(
  //       timeWindowEnd.getMinutes() + 5,
  //     );
  //   });
  //
  //   describe('CREATE transition', () => {
  //     it('should return invalid result if timestamps mismatch - Rust', async () => {
  //       documentTransitionsJs = getDocumentTransitionsFixture({
  //         create: [documentsJs[0]],
  //       });
  //
  //       stateTransition = new DocumentsBatchTransition({
  //         ownerId: ownerIdJs,
  //         contractId: dataContractJs.getId(),
  //         transitions: documentTransitionsJs.map((t) => t.toObject()),
  //       }, [dataContract]);
  //
  //       const transitions = stateTransition.getTransitions();
  //       transitions.forEach((t) => {
  //         // eslint-disable-next-line no-param-reassign
  //         t.setUpdatedAt(new Date());
  //       });
  //       stateTransition.setTransitions(transitions);
  //
  //       const result = await validateDocumentsBatchTransitionState(
  //         stateRepositoryMock, stateTransition, executionContext,
  //       );
  //
  //       expect(result).is.instanceOf(ValidationResult);
  //
  //       const [error] = result.getErrors();
  //       expect(error).is.instanceOf(DocumentTimestampsMismatchError);
  //       expect(error.getCode()).to.equal(4007);
  //     });
  //
  //     it('should return invalid result if "$createdAt" have violated time window  - Rust', async () => {
  //       documentTransitionsJs = getDocumentTransitionsFixture({
  //         create: [documentsJs[0]],
  //       });
  //
  //       stateTransition = new DocumentsBatchTransition({
  //         ownerId: ownerIdJs,
  //         contractId: dataContractJs.getId(),
  //         transitions: documentTransitionsJs.map((t) => t.toObject()),
  //       }, [dataContract]);
  //
  //       const transitions = stateTransition.getTransitions();
  //       transitions.forEach((t) => {
  //         const createdAtMinus6Mins = t.getCreatedAt().getTime() - (6 * 60 * 1000);
  //         t.setCreatedAt(new Date(createdAtMinus6Mins));
  //         t.setUpdatedAt(undefined);
  //       });
  //       stateTransition.setTransitions(transitions);
  //
  //       const result = await validateDocumentsBatchTransitionState(
  //         stateRepositoryMock, stateTransition, executionContext,
  //       );
  //
  //       expect(result).is.instanceOf(ValidationResult);
  //       expect(result.isValid()).is.not.true();
  //
  //       const [error] = result.getErrors();
  //       expect(error).is.instanceOf(DocumentTimestampWindowViolationError);
  //       expect(error.getCode()).to.equal(4008);
  //       expect(error.getTimestampName()).to.equal('createdAt');
  //
  //       expect(error.getTimestamp().getMilliseconds()).to.equal(
  //         documentTransitionsJs[0].createdAt.getMilliseconds(),
  //       );
  //       expect(error.getTimeWindowStart()).to.deep.equal(timeWindowStart);
  //       expect(error.getTimeWindowEnd()).to.deep.equal(timeWindowEnd);
  //     });
  //
  //     it('should return invalid result if "$updatedAt" have violated time window - Rust', async () => {
  //       documentTransitionsJs = getDocumentTransitionsFixture({
  //         create: [documentsJs[1]],
  //       });
  //
  //       stateTransition = new DocumentsBatchTransition({
  //         ownerId: ownerIdJs,
  //         contractId: dataContractJs.getId(),
  //         transitions: documentTransitionsJs.map((t) => t.toObject()),
  //       }, [dataContract]);
  //
  //       const transitions = stateTransition.getTransitions();
  //       transitions.forEach((t) => {
  //         const createdAtMinus6Mins = t.getUpdatedAt().getTime() - (6 * 60 * 1000);
  //         t.setUpdatedAt(new Date(createdAtMinus6Mins));
  //         t.setCreatedAt(undefined);
  //       });
  //       stateTransition.setTransitions(transitions);
  //
  //       const result = await validateDocumentsBatchTransitionState(
  //         stateRepositoryMock, stateTransition, executionContext,
  //       );
  //
  //       expect(result).is.instanceOf(ValidationResult);
  //       expect(result.isValid()).is.not.true();
  //
  //       const [error] = result.getErrors();
  //       expect(error).is.instanceOf(DocumentTimestampWindowViolationError);
  //       expect(error.getCode()).to.equal(4008);
  //
  //       documentTransitionsJs[0].updatedAt.setMinutes(
  //         documentTransitionsJs[0].updatedAt.getMinutes() - 6,
  //       );
  //       documentTransitionsJs[0].createdAt = undefined;
  //
  //       expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());
  //       expect(error.getTimestampName()).to.equal('updatedAt');
  //       expect(error.getTimestamp().getMilliseconds()).to.deep.equal(
  //         documentTransitionsJs[0].updatedAt.getMilliseconds(),
  //       );
  //       expect(error.getTimeWindowStart()).to.deep.equal(timeWindowStart);
  //       expect(error.getTimeWindowEnd()).to.deep.equal(timeWindowEnd);
  //     });
  //
  //     it('should not validate time in block window on dry run - Rust', async () => {
  //       documentTransitionsJs = getDocumentTransitionsFixture({
  //         create: [documentsJs[1]],
  //       });
  //
  //       executeDataTriggersMock.resolves([
  //         new DataTriggerExecutionResult(),
  //       ]);
  //
  //       stateTransition = new DocumentsBatchTransition({
  //         ownerId: ownerIdJs,
  //         contractId: dataContractJs.getId(),
  //         transitions: documentTransitionsJs.map((t) => t.toObject()),
  //       }, [dataContract]);
  //
  //       const transitions = stateTransition.getTransitions();
  //       transitions.forEach((t) => {
  //         const createdAtMinus6Mins = t.getUpdatedAt().getTime() - (6 * 60 * 1000);
  //         t.setUpdatedAt(new Date(createdAtMinus6Mins));
  //         t.setCreatedAt(undefined);
  //       });
  //       stateTransition.setTransitions(transitions);
  //       executionContext.enableDryRun();
  //
  //       const result = await validateDocumentsBatchTransitionState(
  //         stateRepositoryMock, stateTransition, executionContext,
  //       );
  //
  //       expect(result).to.be.an.instanceOf(ValidationResult);
  //       expect(result.isValid()).to.be.true();
  //     });
  //
  //     it('should return valid result if timestamps mismatch on dry run - Rust', async () => {
  //       documentTransitionsJs = getDocumentTransitionsFixture({
  //         create: [documentsJs[0]],
  //       });
  //
  //       executeDataTriggersMock.resolves([
  //         new DataTriggerExecutionResult(),
  //       ]);
  //
  //       stateTransition = new DocumentsBatchTransition({
  //         ownerId: ownerIdJs,
  //         contractId: dataContractJs.getId(),
  //         transitions: documentTransitionsJs.map((t) => t.toObject()),
  //       }, [dataContract]);
  //
  //       const transitions = stateTransition.getTransitions();
  //       transitions.forEach((t) => {
  //         // eslint-disable-next-line no-param-reassign
  //         t.setUpdatedAt(new Date());
  //       });
  //       executionContext.enableDryRun();
  //
  //       const result = await validateDocumentsBatchTransitionState(
  //         stateRepositoryMock, stateTransition, executionContext,
  //       );
  //
  //       expect(result).to.be.an.instanceOf(ValidationResult);
  //       expect(result.isValid()).to.be.true();
  //     });
  //   });
  //
  //   describe('REPLACE transition', () => {
  //     it('should return invalid result if documents with action "replace" have violated time window - Rust', async () => {
  //       documentTransitionsJs = getDocumentTransitionsFixture({
  //         create: [],
  //         replace: [documentsJs[1]],
  //       });
  //
  //       stateTransition = new DocumentsBatchTransition({
  //         ownerId: ownerIdJs,
  //         contractId: dataContractJs.getId(),
  //         transitions: documentTransitionsJs.map((t) => t.toObject()),
  //       }, [dataContract]);
  //
  //       documentsJs[1].updatedAt.setMinutes(
  //         documentsJs[1].updatedAt.getMinutes() - 6,
  //       );
  //
  //       const documentToReturn = new ExtendedDocument(documentsJs[1].toObject(), dataContract);
  //       stateRepositoryMock.fetchExtendedDocuments.resolves([documentToReturn]);
  //
  //       const transitions = stateTransition.getTransitions();
  //       transitions.forEach((t) => {
  //         const createdAtMinus6Mins = t.getUpdatedAt().getTime() - (6 * 60 * 1000);
  //         t.setUpdatedAt(new Date(createdAtMinus6Mins));
  //       });
  //       stateTransition.setTransitions(transitions);
  //
  //       const result = await validateDocumentsBatchTransitionState(
  //         stateRepositoryMock, stateTransition, executionContext,
  //       );
  //
  //       expect(result).is.instanceOf(ValidationResult);
  //       expect(result.isValid()).is.not.true();
  //       const [error] = result.getErrors();
  //
  //       expect(error).is.instanceOf(DocumentTimestampWindowViolationError);
  //       expect(error.getCode()).to.equal(4008);
  //
  //       documentTransitionsJs[0].updatedAt.setMinutes(
  //         documentTransitionsJs[0].updatedAt.getMinutes() - 6,
  //       );
  //
  //       expect(error.getDocumentId()).to.deep.equal(documentTransitionsJs[0].getId().toBuffer());
  //       expect(error.getTimestampName()).to.equal('updatedAt');
  //       expect(error.getTimestamp()).to.deep.equal(documentTransitionsJs[0].updatedAt);
  //       expect(error.getTimeWindowStart()).to.deep.equal(timeWindowStart);
  //       expect(error.getTimeWindowEnd()).to.deep.equal(timeWindowEnd);
  //     });
  //
  //     it('should return valid result if documents with action "replace" have violated time window on dry run - Rust', async () => {
  //       documentTransitionsJs = getDocumentTransitionsFixture({
  //         create: [],
  //         replace: [documentsJs[1]],
  //       });
  //
  //       stateTransition = new DocumentsBatchTransition({
  //         ownerId: ownerIdJs,
  //         contractId: dataContractJs.getId(),
  //         transitions: documentTransitionsJs.map((t) => t.toObject()),
  //       }, [dataContract]);
  //
  //       documentsJs[1].updatedAt.setMinutes(
  //         documentsJs[1].updatedAt.getMinutes() - 6,
  //       );
  //
  //       const documentToReturn = new ExtendedDocument(documentsJs[1].toObject(), dataContract);
  //       stateRepositoryMock.fetchExtendedDocuments.resolves([documentToReturn]);
  //
  //       const transitions = stateTransition.getTransitions(); transitions.forEach((t) => {
  //         const createdAtMinus6Mins = t.getUpdatedAt().getTime() - (6 * 60 * 1000);
  //         t.setUpdatedAt(new Date(createdAtMinus6Mins));
  //       });
  //       stateTransition.setTransitions(transitions);
  //       executionContext.enableDryRun();
  //
  //       const result = await validateDocumentsBatchTransitionState(
  //         stateRepositoryMock, stateTransition, executionContext,
  //       );
  //
  //       expect(result).to.be.an.instanceOf(ValidationResult);
  //       expect(result.isValid()).to.be.true();
  //     });
  //   });
  // });
  //
  // it('should return valid result if document transitions are valid - Rust', async () => {
  //   const fetchedDocuments = [
  //     new ExtendedDocument(documentsJs[1].toObject(), dataContract).getDocument(),
  //     new ExtendedDocument(documentsJs[2].toObject(), dataContract).getDocument(),
  //   ];
  //
  //   stateRepositoryMock.fetchDocuments.resolves(fetchedDocuments);
  //
  //   documentsJs[1].setRevision(1);
  //   documentsJs[2].setRevision(1);
  //
  //   documentTransitionsJs = getDocumentTransitionsFixture({
  //     create: [],
  //     replace: [documentsJs[1]],
  //     delete: [documentsJs[2]],
  //   });
  //
  //   stateTransition = new DocumentsBatchTransition({
  //     ownerId: ownerIdJs,
  //     contractId: dataContractJs.getId(),
  //     transitions: documentTransitionsJs.map((t) => t.toObject()),
  //   }, [dataContract]);
  //
  //   const result = await validateDocumentsBatchTransitionState(
  //     stateRepositoryMock, stateTransition, executionContext,
  //   );
  //
  //   expect(result).to.be.an.instanceOf(ValidationResult);
  //   expect(result.isValid()).to.be.true();
  //
  //   expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
  //   const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
  //   expect(fetchDataContractId.toBuffer()).to.deep.equal(dataContract.getId().toBuffer());
  //
  //   expect(stateRepositoryMock.fetchDocuments).to.have.been.calledOnce();
  // });
  //
  // it('should return valid result if document transitions are valid - Rust', async () => {
  //   const fetchedDocuments = [
  //     new ExtendedDocument(documentsJs[1].toObject(), dataContract).getDocument(),
  //     new ExtendedDocument(documentsJs[2].toObject(), dataContract).getDocument(),
  //   ];
  //
  //   stateRepositoryMock.fetchDocuments.resolves(fetchedDocuments);
  //
  //   documentsJs[1].setRevision(1);
  //   documentsJs[2].setRevision(1);
  //
  //   documentTransitionsJs = getDocumentTransitionsFixture({
  //     create: [],
  //     replace: [documentsJs[1]],
  //     delete: [documentsJs[2]],
  //   });
  //
  //   stateTransition = new DocumentsBatchTransition({
  //     ownerId: ownerIdJs,
  //     contractId: dataContractJs.getId(),
  //     transitions: documentTransitionsJs.map((t) => t.toObject()),
  //   }, [dataContract]);
  //
  //   const result = await validateDocumentsBatchTransitionState(
  //     stateRepositoryMock, stateTransition, executionContext,
  //   );
  //
  //   expect(result).to.be.an.instanceOf(ValidationResult);
  //   expect(result.isValid()).to.be.true();
  //
  //   expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
  //   const [fetchDataContractId] = stateRepositoryMock.fetchDataContract.getCall(0).args;
  //   expect(fetchDataContractId.toBuffer()).to.deep.equal(dataContract.getId().toBuffer());
  //
  //   expect(stateRepositoryMock.fetchDocuments).to.have.been.calledOnce();
  // });
});
