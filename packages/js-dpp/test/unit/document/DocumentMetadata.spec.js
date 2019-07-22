const DocumentMetadata = require('../../../lib/document/DocumentMetadata');

describe('DocumentMetadata', () => {
  let userId;
  let stReference;
  let rawDocumentMetadata;
  let documentMetadata;

  beforeEach(() => {
    userId = 'test';

    stReference = {
      blockHash: 'someBlockHash',
      blockHeight: 42,
      stHeaderHash: 'someHeaderHash',
      stPacketHash: 'somePacketHash',
    };

    rawDocumentMetadata = {
      userId,
      stReference,
    };

    documentMetadata = new DocumentMetadata(rawDocumentMetadata);
  });

  describe('constructor', () => {
    it('should create DocumentMetadata with $userId if present', () => {
      expect(documentMetadata.userId).to.equal(userId);
    });

    it('should create DocumentMetadata with `stReference` if present', () => {
      expect(documentMetadata.stReference).to.deep.equal(stReference);
    });
  });

  describe('#getUserId', () => {
    it('should return user ID', () => {
      expect(documentMetadata.getUserId()).to.equal(userId);
    });
  });

  describe('#getSTReference', () => {
    it('should return the stReference', () => {
      expect(documentMetadata.getSTReference()).to.deep.equal(stReference);
    });
  });

  describe('#toJSON', () => {
    it('should return DocumentMetadata as plain JS object', () => {
      expect(documentMetadata.toJSON()).to.deep.equal(rawDocumentMetadata);
    });
  });
});
