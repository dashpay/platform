const Document = require('@dashevo/dpp/lib/document/Document');
const SVDocument = require('../../../../lib/stateView/document/SVDocument');

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

  beforeEach(function beforeEach() {
    svDocumentRepository = {
      find: this.sinon.stub(),
      store: this.sinon.stub(),
    };

    ({ userId } = getDocumentsFixture);
    [document] = getDocumentsFixture();

    const createSVDocumentRepository = () => svDocumentRepository;

    updateSVDocument = updateSVDocumentFactory(createSVDocumentRepository);

    reference = getReferenceFixture();
  });

  it('should store SVDocument if action is "create"', async () => {
    await updateSVDocument(document, reference);

    expect(svDocumentRepository.store).to.have.been.calledOnce();

    const svDocument = svDocumentRepository.store.getCall(0).args[0];

    expect(svDocument).to.be.an.instanceOf(SVDocument);
    expect(svDocument.getUserId()).to.equal(userId);
    expect(svDocument.getDocument()).to.equal(document);
    expect(svDocument.getReference()).to.equal(reference);
    expect(svDocument.getPreviousRevisions()).to.deep.equal([]);
    expect(svDocument.isDeleted()).to.be.false();
  });

  it('should store SVDocument if action is "replace" and it has a previous version', async () => {
    const [previousSVDocument] = getSVDocumentsFixture();

    svDocumentRepository.find.returns(previousSVDocument);

    document.setRevision(1);
    document.setAction(Document.ACTIONS.REPLACE);

    await updateSVDocument(document, reference);

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

  it('should throw an error if action is "replace" and there is no previous version', async () => {
    svDocumentRepository.find.returns(null);

    document.setAction(Document.ACTIONS.REPLACE);

    let error;
    try {
      await updateSVDocument(document, reference);
    } catch (e) {
      error = e;
    }

    expect(error).to.be.an.instanceOf(Error);

    expect(svDocumentRepository.find).to.have.been.calledOnceWith(document.getId());
    expect(svDocumentRepository.store).to.have.not.been.called();
  });

  it('should delete SVDocument if action is "delete"', async () => {
    const [previousSVDocument] = getSVDocumentsFixture();

    svDocumentRepository.find.returns(previousSVDocument);

    document.setRevision(1);
    document.setData({});
    document.setAction(Document.ACTIONS.DELETE);

    await updateSVDocument(document, reference);

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
      await updateSVDocument(document, reference);
    } catch (e) {
      error = e;
    }

    expect(error).to.be.an.instanceOf(Error);

    expect(svDocumentRepository.store).to.have.not.been.called();
  });
});
