const findDuplicateDocuments = require('../../../../../lib/document/stateTransition/validation/structure/findDuplicateDocumentsById');

const getDocumentsFixture = require('../../../../../lib/test/fixtures/getDocumentsFixture');

describe('findDuplicateDocumentsById', () => {
  let documents;

  beforeEach(() => {
    documents = getDocumentsFixture();
  });

  it('should return empty array if there are no duplicated Documents', () => {
    const result = findDuplicateDocuments(documents);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(0);
  });

  it('should return duplicated Documents', () => {
    documents.push(documents[0]);

    const result = findDuplicateDocuments(documents);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(2);
    expect(result).to.have.deep.members([
      documents[0].toJSON(),
      documents[0].toJSON(),
    ]);
  });
});
