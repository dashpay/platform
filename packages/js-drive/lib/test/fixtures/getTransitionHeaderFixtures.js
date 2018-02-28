const fs = require('fs');
const path = require('path');

const StateTransitionHeader = require('../../blockchain/StateTransitionHeader');

let transitionHeaders;

/**
 * @return {StateTransitionHeader[]}
 */
module.exports = function getTransitionHeaderFixtures() {
  if (!transitionHeaders) {
    const transitionHeadersJSON = fs.readFileSync(path.join(__dirname, '/../../../test/fixtures/stateTransitionHeaders.json'));
    const transitionHeadersData = JSON.parse(transitionHeadersJSON);

    transitionHeaders = transitionHeadersData.map(h => new StateTransitionHeader(h));
  }

  return transitionHeaders;
};
