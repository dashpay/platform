const verifyDocumentsFactory = require('../../../../lib/stPacket/verification/verifyDocumentsFactory');

const STPacket = require('../../../../lib/stPacket/STPacket');
const Document = require('../../../../lib/document/Document');

const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');
const getDPContractFixture = require('../../../../lib/test/fixtures/getDPContractFixture');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const InvalidDocumentScopeError = require('../../../../lib/errors/InvalidDocumentScopeError');
const DocumentAlreadyPresentError = require('../../../../lib/errors/DocumentAlreadyPresentError');
const DocumentNotFoundError = require('../../../../lib/errors/DocumentNotFoundError');
const InvalidDocumentRevisionError = require('../../../../lib/errors/InvalidDocumentRevisionError');
const InvalidDocumentActionError = require('../../../../lib/stPacket/errors/InvalidDocumentActionError');

describe('verifyDocuments', () => {
  let verifyDocuments;
  let fetchDocumentsByDocumentsMock;
  let stPacket;
  let documents;
  let dpContract;
  let userId;
  let verifyDocumentsUniquenessByIndices;

  beforeEach(function beforeEach() {
    ({ userId } = getDocumentsFixture);

    documents = getDocumentsFixture();
    dpContract = getDPContractFixture();

    stPacket = new STPacket(dpContract.getId());
    stPacket.setDocuments(documents);

    fetchDocumentsByDocumentsMock = this.sinonSandbox.stub();

    verifyDocumentsUniquenessByIndices = this.sinonSandbox.stub();
    verifyDocumentsUniquenessByIndices.resolves(new ValidationResult());

    verifyDocuments = verifyDocumentsFactory(
      fetchDocumentsByDocumentsMock,
      verifyDocumentsUniquenessByIndices,
    );
  });

  it('should return invalid result if Document has wrong scope', async () => {
    documents[0].scope = 'wrong';

    fetchDocumentsByDocumentsMock.resolves([]);

    const result = await verifyDocuments(stPacket, userId, dpContract);

    expectValidationError(result, InvalidDocumentScopeError);

    expect(fetchDocumentsByDocumentsMock).to.have.been.calledOnceWith(
      stPacket.getDPContractId(),
      documents,
    );

    const [error] = result.getErrors();

    expect(error.getDocument()).to.equal(documents[0]);
  });

  it('should return invalid result if Document with action "create" is already present', async () => {
    fetchDocumentsByDocumentsMock.resolves([documents[0]]);

    const result = await verifyDocuments(stPacket, userId, dpContract);

    expectValidationError(result, DocumentAlreadyPresentError);

    expect(fetchDocumentsByDocumentsMock).to.have.been.calledOnceWith(
      stPacket.getDPContractId(),
      documents,
    );

    const [error] = result.getErrors();

    expect(error.getDocument()).to.equal(documents[0]);
    expect(error.getFetchedDocument()).to.equal(documents[0]);
  });

  it('should return invalid result if Document with action "update" is not present', async () => {
    documents[0].setAction(Document.ACTIONS.UPDATE);

    fetchDocumentsByDocumentsMock.resolves([]);

    const result = await verifyDocuments(stPacket, userId, dpContract);

    expectValidationError(result, DocumentNotFoundError);

    expect(fetchDocumentsByDocumentsMock).to.have.been.calledOnceWith(
      stPacket.getDPContractId(),
      documents,
    );

    const [error] = result.getErrors();

    expect(error.getDocument()).to.equal(documents[0]);
  });

  it('should return invalid result if Document with action "delete" is not present', async () => {
    documents[0].setData({});
    documents[0].setAction(Document.ACTIONS.DELETE);

    fetchDocumentsByDocumentsMock.resolves([]);

    const result = await verifyDocuments(stPacket, userId, dpContract);

    expectValidationError(result, DocumentNotFoundError);

    expect(fetchDocumentsByDocumentsMock).to.have.been.calledOnceWith(
      stPacket.getDPContractId(),
      documents,
    );

    const [error] = result.getErrors();

    expect(error.getDocument()).to.equal(documents[0]);
  });

  it('should return invalid result if Document with action "update" has wrong revision', async () => {
    documents[0].setAction(Document.ACTIONS.UPDATE);

    fetchDocumentsByDocumentsMock.resolves([documents[0]]);

    const result = await verifyDocuments(stPacket, userId, dpContract);

    expectValidationError(result, InvalidDocumentRevisionError);

    expect(fetchDocumentsByDocumentsMock).to.have.been.calledOnceWith(
      stPacket.getDPContractId(),
      documents,
    );

    const [error] = result.getErrors();

    expect(error.getDocument()).to.equal(documents[0]);
    expect(error.getFetchedDocument()).to.equal(documents[0]);
  });

  it('should return invalid result if Document with action "delete" has wrong revision', async () => {
    documents[0].setData({});
    documents[0].setAction(Document.ACTIONS.DELETE);

    fetchDocumentsByDocumentsMock.resolves([documents[0]]);

    const result = await verifyDocuments(stPacket, userId, dpContract);

    expectValidationError(result, InvalidDocumentRevisionError);

    expect(fetchDocumentsByDocumentsMock).to.have.been.calledOnceWith(
      stPacket.getDPContractId(),
      documents,
    );

    const [error] = result.getErrors();

    expect(error.getDocument()).to.equal(documents[0]);
  });

  it('should throw an error if Document has invalid action', async () => {
    documents[0].setAction(5);

    fetchDocumentsByDocumentsMock.resolves([documents[0]]);

    let error;
    try {
      await verifyDocuments(stPacket, userId, dpContract);
    } catch (e) {
      error = e;
    }

    expect(error).to.be.an.instanceOf(InvalidDocumentActionError);
    expect(error.getDocument()).to.equal(documents[0]);
  });

  it('should return valid result if Documents are valid', async () => {
    const fetchedDocuments = [
      new Document(documents[1].toJSON()),
      new Document(documents[2].toJSON()),
    ];

    fetchDocumentsByDocumentsMock.resolves(fetchedDocuments);

    documents[1].setAction(Document.ACTIONS.UPDATE);
    documents[1].setRevision(1);

    documents[2].setData({});
    documents[2].setAction(Document.ACTIONS.DELETE);
    documents[2].setRevision(1);

    const result = await verifyDocuments(stPacket, userId, dpContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
