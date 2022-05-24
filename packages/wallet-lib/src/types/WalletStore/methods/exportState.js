function exportState(chainHeight) {
  const { walletId } = this.state;
  let { lastKnownBlock: { height } } = this.state;

  /*
   * If we have chain height provided, we must set last known block to
   * chainHeight - 6 to avoid reorgs
   */
  if (chainHeight && height > chainHeight - 6) {
    height = chainHeight - 6;
  }

  return {
    walletId,
    lastKnownBlock: {
      height,
    },
  };
}
module.exports = exportState;
