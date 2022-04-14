const castItemTypes = require('../../../utils/castItemTypes');

function importState(rawState) {
  const state = castItemTypes(rawState, this.SCHEMA);

  this.state.lastKnownBlock = state.lastKnownBlock;
}

module.exports = importState;
