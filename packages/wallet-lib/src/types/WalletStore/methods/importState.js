function importState(state) {
  this.state.lastKnownBlock = state.lastKnownBlock;
}
module.exports = importState;
