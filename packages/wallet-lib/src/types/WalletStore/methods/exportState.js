function exportState() {
  const { lastKnownBlock } = this.state;

  return {
    lastKnownBlock: {
      ...lastKnownBlock,
    },
  };
}
module.exports = exportState;
