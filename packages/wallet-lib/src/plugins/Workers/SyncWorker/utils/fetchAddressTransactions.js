module.exports = async function fetchAndStoreAddressTransactions(address, transport) {
  const promises = [];
  const summary = await transport.getAddressSummary(address);

  if (summary.transactions.length) {
    summary.transactions
      .forEach((txid) => {
        promises.push(transport.getTransaction(txid));
      });
  }

  return Promise.all(promises);
};
