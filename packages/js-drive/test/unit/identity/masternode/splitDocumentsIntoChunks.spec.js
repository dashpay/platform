const documentsBatchSchema = require('@dashevo/dpp/schema/document/stateTransition/documentsBatch.json');
const splitDocumentsIntoChunks = require('../../../../lib/identity/masternode/splitDocumentsIntoChunks');

describe('splitDocumentsIntoChunks', () => {
  let documents;
  let chunkLength;

  beforeEach(() => {
    documents = {};

    chunkLength = documentsBatchSchema.properties.transitions.maxItems;
  });

  it('should return empty result if documents are empty', () => {
    documents = {
      create: [],
      delete: [],
    };
    const result = splitDocumentsIntoChunks(documents);

    expect(result).to.deep.equal([documents]);
  });

  it('should return same documents if amount < maxAmount', () => {
    const result = splitDocumentsIntoChunks(documents);

    expect(result).to.deep.equal([documents]);
  });

  it('should split create documents', () => {
    documents.create = Array.from(
      { length: chunkLength * 2 + 1 },
      () => Math.floor(Math.random() * 40),
    );

    const result = splitDocumentsIntoChunks(documents);

    expect(result).to.have.lengthOf(3);
    expect(result[0].delete).to.deep.equal(documents.delete);
    expect(result[1].delete).to.deep.equal(undefined);
    expect(result[2].delete).to.deep.equal(undefined);

    expect(result[0].create).to.deep.equal(documents.create.slice(0, chunkLength));
    expect(result[1].create).to.deep.equal(documents.create.slice(chunkLength, chunkLength * 2));
    expect(result[2].create).to.deep.equal(
      documents.create.slice(chunkLength * 2, chunkLength * 3 + 1),
    );
  });

  it('should split delete documents', () => {
    documents.delete = Array.from(
      { length: chunkLength * 2 + 1 },
      () => Math.floor(Math.random() * 40),
    );

    const result = splitDocumentsIntoChunks(documents);

    expect(result).to.have.lengthOf(3);
    expect(result[0].create).to.deep.equal(documents.create);
    expect(result[1].create).to.deep.equal(undefined);
    expect(result[2].create).to.deep.equal(undefined);

    expect(result[0].delete).to.deep.equal(documents.delete.slice(0, chunkLength));
    expect(result[1].delete).to.deep.equal(documents.delete.slice(chunkLength, chunkLength * 2));
    expect(result[2].delete).to.deep.equal(
      documents.delete.slice(chunkLength * 2, chunkLength * 3 + 1),
    );
  });

  it('should split create and delete documents', () => {
    documents.create = Array.from(
      { length: chunkLength * 2 + 1 },
      () => Math.floor(Math.random() * 40),
    );

    documents.delete = Array.from(
      { length: chunkLength * 2 + 1 },
      () => Math.floor(Math.random() * 40),
    );

    const result = splitDocumentsIntoChunks(documents);
    expect(result).to.have.lengthOf(5);

    result.forEach((chunk) => {
      let ducumentsAmount = 0;
      if (chunk.create) {
        ducumentsAmount += chunk.create.length;
      }

      if (chunk.delete) {
        ducumentsAmount += chunk.delete.length;
      }

      expect(ducumentsAmount).to.be.lessThanOrEqual(chunkLength);
    });
  });
});
