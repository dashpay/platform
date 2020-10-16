const DocumentFactory = require('../../document/DocumentFactory');

const getDocumentsFixture = require('./getDocumentsFixture');

function getDocumentTransitionsFixture(documents = {}) {
  const {
    create: createDocuments,
    replace: replaceDocuments,
    delete: deleteDocuments,
  } = documents;

  const fixtureDocuments = getDocumentsFixture();

  const factory = new DocumentFactory(() => {}, () => {});

  const stateTransition = factory.createStateTransition({
    create: (createDocuments || fixtureDocuments),
    replace: (replaceDocuments || []),
    delete: (deleteDocuments || []),
  });

  return stateTransition.getTransitions();
}

module.exports = getDocumentTransitionsFixture;
