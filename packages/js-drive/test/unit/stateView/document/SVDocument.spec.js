const SVDocument = require('../../../../lib/stateView/document/SVDocument');

const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');
const getReferenceFixture = require('../../../../lib/test/fixtures/getReferenceFixture');

describe('SVDocument', () => {
  let svDocument;
  let userId;
  let document;
  let reference;
  let isDeleted;
  let previousRevisions;

  beforeEach(() => {
    ({ userId } = getDocumentsFixture);
    [document] = getDocumentsFixture();
    reference = getReferenceFixture();
    isDeleted = false;
    previousRevisions = [];

    svDocument = new SVDocument(
      userId,
      document,
      reference,
      isDeleted,
      previousRevisions,
    );
  });

  describe('#getUserId', () => {
    it('should return user ID', () => {
      const result = svDocument.getUserId();

      expect(result).to.equal(userId);
    });
  });

  describe('#getDocument', () => {
    it('should return Document', () => {
      const result = svDocument.getDocument();

      expect(result.toJSON()).to.deep.equal(document.toJSON());
    });
  });

  describe('#markAsDeleted', () => {
    it('should mark document as deleted', () => {
      const result = svDocument.markAsDeleted();

      expect(result).to.equal(svDocument);

      expect(svDocument.deleted).to.be.true();
    });
  });

  describe('#isDeleted', () => {
    it('should return true if document is deleted', () => {
      const result = svDocument.isDeleted();

      expect(result).to.be.false();
    });
  });

  describe('#toJSON', () => {
    it('should return SVDocument as a plain object', () => {
      const result = svDocument.toJSON();

      expect(result).to.deep.equal({
        userId,
        data: document.getData(),
        reference: reference.toJSON(),
        isDeleted,
        previousRevisions,
        scope: document.scope,
        scopeId: document.scopeId,
        action: document.getAction(),
        currentRevision: svDocument.getCurrentRevision().toJSON(),
      });
    });
  });
});
