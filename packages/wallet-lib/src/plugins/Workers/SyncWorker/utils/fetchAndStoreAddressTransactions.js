module.exports = async function fetchAndStoreAddressTransactions(address, transporter, storage) {
  const promises = [];
  const summary = await transporter.getAddressSummary(address);

  if (summary.transactions.length) {
    summary.transactions
      .forEach((txid) => {
        promises.push(transporter.getTransaction(txid));
      });
  }

  await Promise
    .all(promises)
    .then((transactions) => {
      transactions.map((transaction) => storage.importTransaction(transaction));
    });
};
