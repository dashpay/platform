const { Transaction } = require('@dashevo/dashcore-lib');
const fs = require('fs');

module.exports = async function getTransaction(transactionHash) {
  const txFile = JSON.parse(fs.readFileSync(`./fixtures/FakeNet/data/transactions/${transactionHash}.json`));
  return new Transaction(Buffer.from(txFile.transaction, 'hex'));

};
