const DataContractStateTransition = require(
  '@dashevo/dpp/lib/dataContract/stateTransition/DataContractStateTransition',
);

const DocumentsStateTransition = require(
  '@dashevo/dpp/lib/document/stateTransition/DocumentsStateTransition',
);


const getDataContractFixture = require('./getDataContractFixture');
const getDocumentsFixture = require('./getDocumentsFixture');

/**
 * Get some state transition fixtures
 *
 * @return {AbstractStateTransition[]}
 */
function getStateTransitionsFixture() {
  const dataContract = getDataContractFixture();
  const documents = getDocumentsFixture();

  const stateTransitions = [
    new DataContractStateTransition(dataContract),
    new DocumentsStateTransition(documents),
    new DocumentsStateTransition(documents),
  ];

  return stateTransitions;
}

module.exports = getStateTransitionsFixture;
