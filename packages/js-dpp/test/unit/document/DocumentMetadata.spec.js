const DocumentMetadata = require('../../../lib/document/DocumentMetadata');

describe('DocumentMetadata', () => {
  let userId;
  let rawDocumentMetadata;
  let documentMetadata;

  beforeEach(() => {
    userId = 'test';

    rawDocumentMetadata = {
      userId,
    };

    documentMetadata = new DocumentMetadata(rawDocumentMetadata);
  });

  describe('constructor', () => {
    it('should create DocumentMetadata with $userId if present', () => {
      expect(documentMetadata.userId).to.equal(userId);
    });
  });

  describe('#getUserId', () => {
    it('should return user ID', () => {
      expect(documentMetadata.getUserId()).to.equal(userId);
    });
  });

  describe('#toJSON', () => {
    it('should return DocumentMetadata as plain JS object', () => {
      expect(documentMetadata.toJSON()).to.deep.equal(rawDocumentMetadata);
    });
  });
});
