const crypto = require('crypto');
const { DocumentFactory } = require('../../..');

const getDocumentsFixture = require('./getDocumentsFixture');

async function getDocumentTransitionsFixture(documents = {}) {
  const {
    create: createDocuments,
    replace: replaceDocuments,
    delete: deleteDocuments,
  } = documents;

  const fixtureDocuments = await getDocumentsFixture();

  const entropyGenerator = {
    generate() {
      return crypto.randomBytes(32);
    },
  };
  const factory = new DocumentFactory(1, entropyGenerator);

  const stateTransition = factory.createStateTransition({
    create: (createDocuments || fixtureDocuments),
    replace: (replaceDocuments || []),
    delete: (deleteDocuments || []),
  });

  return stateTransition.getTransitions();
}

module.exports = getDocumentTransitionsFixture;
