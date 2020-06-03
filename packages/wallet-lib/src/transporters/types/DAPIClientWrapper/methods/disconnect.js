module.exports = async function disconnect() {
  const { executors } = this.state;
  clearInterval(executors.blocks);
  clearInterval(executors.blockHeaders);
  clearInterval(executors.addresses);
};
