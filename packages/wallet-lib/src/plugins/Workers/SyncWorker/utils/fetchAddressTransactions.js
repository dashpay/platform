module.exports = async function fetchAndStoreAddressTransactions(address, transporter) {
  const promises = [];
  const summary = await transporter.getAddressSummary(address);

  if (summary.transactions.length) {
    summary.transactions
      .forEach((txid) => {
        promises.push(transporter.getTransaction(txid));
      });
  }

  return Promise.all(promises);
};
