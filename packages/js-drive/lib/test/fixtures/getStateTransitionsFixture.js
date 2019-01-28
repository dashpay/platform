const fs = require('fs');
const path = require('path');

const StateTransition = require('../../blockchain/StateTransition');

/**
 * @return {StateTransition[]}
 */
module.exports = function getStateTransitionsFixture() {
  const stateTransitionsJSON = fs.readFileSync(path.join(__dirname, '/../../../test/fixtures/stateTransitions.json'));
  const stateTransitionsData = JSON.parse(stateTransitionsJSON.toString());

  return stateTransitionsData.map(h => new StateTransition(h));
};
