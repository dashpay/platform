const Document = require('@dashevo/dpp/lib/document/Document');
const SVDocument = require('../../../../lib/stateView/document/SVDocument');

const Revision = require('../../../../lib/stateView/revisions/Revision');

const updateSVDocumentFactory = require('../../../../lib/stateView/document/updateSVDocumentFactory');

const getReferenceFixture = require('../../../../lib/test/fixtures/getReferenceFixture');
const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');
const getSVDocumentsFixture = require('../../../../lib/test/fixtures/getSVDocumentsFixture');

describe('updateSVDocumentFactory', () => {
  let svDocumentRepository;
  let updateSVDocument;
  let reference;
  let document;
  let userId;
  let contractId;

  beforeEach(function beforeEach() {
    svDocumentRepository = {
      find: this.sinon.stub(),
      store: this.sinon.stub(),
    };

    contractId = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    ({ userId } = getDocumentsFixture);
    [document] = getDocumentsFixture();

    const createSVDocumentRepository = () => svDocumentRepository;

    updateSVDocument = updateSVDocumentFactory(createSVDocumentRepository);

    reference = getReferenceFixture();
  });

  it('should store SVDocument if action is "create"', async () => {
    await updateSVDocument(contractId, userId, reference, document);

    expect(svDocumentRepository.store).to.have.been.calledOnce();

    const svDocument = svDocumentRepository.store.getCall(0).args[0];

    expect(svDocument).to.be.an.instanceOf(SVDocument);
    expect(svDocument.getUserId()).to.equal(userId);
    expect(svDocument.getDocument()).to.equal(document);
    expect(svDocument.getReference()).to.equal(reference);
    expect(svDocument.getPreviousRevisions()).to.deep.equal([]);
    expect(svDocument.isDeleted()).to.be.false();
  });

  it('should store SVDocument if action is "update" and it has a previous version', async () => {
    const [previousSVDocument] = getSVDocumentsFixture();

    svDocumentRepository.find.returns(previousSVDocument);

    document.setRevision(1);
    document.setAction(Document.ACTIONS.UPDATE);

    await updateSVDocument(contractId, userId, reference, document);

    expect(svDocumentRepository.find).to.have.been.calledOnceWith(document.getId());
    expect(svDocumentRepository.store).to.have.been.calledOnce();

    const svDocument = svDocumentRepository.store.getCall(0).args[0];

    expect(svDocument).to.be.an.instanceOf(SVDocument);
    expect(svDocument.getUserId()).to.equal(userId);
    expect(svDocument.getDocument()).to.equal(document);
    expect(svDocument.getReference()).to.equal(reference);
    expect(svDocument.getPreviousRevisions()).to.deep.equal([
      previousSVDocument.getCurrentRevision(),
    ]);
    expect(svDocument.isDeleted()).to.be.false();
  });

  it('should throw an error if action is "update" and there is no previous version', async () => {
    svDocumentRepository.find.returns(null);

    document.setAction(Document.ACTIONS.UPDATE);

    let error;
    try {
      await updateSVDocument(contractId, userId, reference, document);
    } catch (e) {
      error = e;
    }

    expect(error).to.be.an.instanceOf(Error);

    expect(svDocumentRepository.find).to.have.been.calledOnceWith(document.getId());
    expect(svDocumentRepository.store).to.have.not.been.called();
  });

  it('should store SVDocument and remove ahead versions if action is "update" upon reverting', async () => {
    const previousRevisions = [
      new Revision(0, reference),
      new Revision(1, reference),
      new Revision(2, reference),
      new Revision(3, reference),
    ];

    const isDeleted = false;

    const previousSVDocument = new SVDocument(
      userId,
      document,
      reference,
      isDeleted,
      previousRevisions,
    );

    svDocumentRepository.find.returns(previousSVDocument);

    document.setAction(Document.ACTIONS.UPDATE);
    document.setRevision(2);

    await updateSVDocument(contractId, userId, reference, document, true);

    expect(svDocumentRepository.find).to.have.been.calledOnceWith(document.getId());
    expect(svDocumentRepository.store).to.have.been.calledOnce();

    const svDocument = svDocumentRepository.store.getCall(0).args[0];

    expect(svDocument).to.be.an.instanceOf(SVDocument);
    expect(svDocument.getUserId()).to.equal(userId);
    expect(svDocument.getDocument()).to.equal(document);
    expect(svDocument.getReference()).to.equal(reference);
    expect(svDocument.getPreviousRevisions()).to.deep.equal(previousRevisions.slice(0, 2));
    expect(svDocument.isDeleted()).to.be.false();
  });

  it('should delete SVDocument if action is "delete"', async () => {
    const [previousSVDocument] = getSVDocumentsFixture();

    svDocumentRepository.find.returns(previousSVDocument);

    document.setRevision(1);
    document.setData({});
    document.setAction(Document.ACTIONS.DELETE);

    await updateSVDocument(contractId, userId, reference, document);

    expect(svDocumentRepository.store).to.have.been.calledOnce();

    const svDocument = svDocumentRepository.store.getCall(0).args[0];

    expect(svDocument).to.be.an.instanceOf(SVDocument);
    expect(svDocument.getUserId()).to.equal(userId);
    expect(svDocument.getDocument()).to.equal(document);
    expect(svDocument.getReference()).to.equal(reference);
    expect(svDocument.getPreviousRevisions()).to.deep.equal([
      previousSVDocument.getCurrentRevision(),
    ]);
    expect(svDocument.isDeleted()).to.be.true();
  });


  it('should throw an error if action is not supported', async () => {
    document.setAction(100);

    let error;
    try {
      await updateSVDocument(contractId, userId, reference, document);
    } catch (e) {
      error = e;
    }

    expect(error).to.be.an.instanceOf(Error);

    expect(svDocumentRepository.store).to.have.not.been.called();
  });
});
