const findDuplicateDocuments = require('../../../../../lib/document/stateTransition/validation/structure/findDuplicatesById');

const getDocumentTransitionsFixture = require('../../../../../lib/test/fixtures/getDocumentTransitionsFixture');

describe('findDuplicatesById', () => {
  let documentTransitions;

  beforeEach(() => {
    documentTransitions = getDocumentTransitionsFixture().map((t) => t.toJSON());
  });

  it('should return empty array if there are no duplicated Documents', () => {
    const result = findDuplicateDocuments(documentTransitions);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(0);
  });

  it('should return duplicated Documents', () => {
    documentTransitions.push(documentTransitions[0]);

    const result = findDuplicateDocuments(documentTransitions);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(2);
    expect(result).to.have.deep.members([
      documentTransitions[0],
      documentTransitions[0],
    ]);
  });
});
