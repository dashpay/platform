const DocumentFactory = require('../../document/DocumentFactory');
const createDPPMock = require('../mocks/createDPPMock');

const getDocumentsFixture = require('./getDocumentsFixture');

function getDocumentTransitionsFixture(documents = {}) {
  const {
    create: createDocuments,
    replace: replaceDocuments,
    delete: deleteDocuments,
  } = documents;

  const fixtureDocuments = getDocumentsFixture();

  const factory = new DocumentFactory(
    createDPPMock(),
    () => {},
    () => {},
  );

  const stateTransition = factory.createStateTransition({
    create: (createDocuments || fixtureDocuments),
    replace: (replaceDocuments || []),
    delete: (deleteDocuments || []),
  });

  return stateTransition.getTransitions();
}

module.exports = getDocumentTransitionsFixture;
