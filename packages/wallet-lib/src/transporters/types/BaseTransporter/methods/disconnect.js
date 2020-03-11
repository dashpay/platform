module.exports = function disconnect() {
  clearInterval(this.state.subscriptions.blocks);
  clearInterval(this.state.subscriptions.blockHeaders);
  // eslint-disable-next-line guard-for-in,no-restricted-syntax
  for (const addr in this.state.subscriptions.addresses) {
    clearInterval(addr);
    delete this.state.subscriptions.addresses[addr];
  }
  return true;
};
