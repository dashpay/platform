const SVDocument = require('../../stateView/document/SVDocument');

const getDocumentsFixture = require('./getDocumentsFixture');
const getReferenceFixture = require('./getReferenceFixture');

/**
 * @return {SVDocument[]}
 */
function getSVDocumentsFixture() {
  const { userId } = getDocumentsFixture;
  const documents = getDocumentsFixture();

  return documents.map((document, i) => new SVDocument(
    userId,
    document,
    getReferenceFixture(i + 1),
  ));
}

module.exports = getSVDocumentsFixture;
