function exportState() {
  const { lastKnownBlock } = this.state;

  return {
    lastKnownBlock,
  };
}
module.exports = exportState;
