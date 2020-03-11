module.exports = async function disconnect() {
  const { subscriptions } = this.state;
  clearInterval(subscriptions.blocks);
  clearInterval(subscriptions.blockHeaders);
  // eslint-disable-next-line guard-for-in,no-restricted-syntax
  for (const addr in subscriptions.addresses) {
    clearInterval(subscriptions.addresses[addr]);
  }
};
