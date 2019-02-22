const StateTransition = require('../../blockchain/StateTransition');

const createPayloadFixture = require('./createPayloadFixture');
const createStateTransitionFixture = require('./createStateTransitionFixture');

const transitions = [];

/**
 * @param {integer} numberOfTransitions
 * @return {StateTransition[]}
 */
function getStateTransitionsFixture(numberOfTransitions = 5) {
  if (transitions.length > 0) {
    return transitions.map(t => new StateTransition(t));
  }

  for (let i = 0; i < numberOfTransitions; i++) {
    transitions.push(
      createStateTransitionFixture({
        extraPayload: createPayloadFixture({
          hashPrevSubTx: (i === 0 ? undefined : transitions[i - 1].hash),
        }),
      }),
    );
  }

  return transitions.map(t => new StateTransition(t));
}

module.exports = getStateTransitionsFixture;
