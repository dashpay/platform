const { Transaction } = require('@dashevo/dashcore-lib');
const fs = require('fs');
const { is } = require('../../../src/utils');

function getUtxoOfAddressAtHeight(address, height) {
  let utxos = [];
  const path = `./fixtures/FakeNet/data/utxos/${address}.json`;
  if (!fs.existsSync(path)) {
    return utxos;
  }
  const utxofile = JSON.parse(fs.readFileSync(path));
  for (let i = parseInt(Object.keys(utxofile)[0], 10) - 1; i <= height; i += 1) {
    const el = utxofile[i];
    if (el) {
      utxos = utxos.concat(el);
    }
  }
  return utxos.map((utxo)=> {
    return new Transaction.UnspentOutput(utxo)
  });;
}

module.exports = async function getUTXO(addresses) {
  if (!is.address(addresses) && !is.arr(addresses)) throw new Error('Received an invalid address to fetch');
  let utxos = [];
  if (is.arr(addresses)) {
    // eslint-disable-next-line no-restricted-syntax
    for (const address of addresses) {
      // eslint-disable-next-line no-await-in-loop
      utxos = utxos.concat(await getUTXO.call(this, address));
    }
  } else {
    const { height } = this;
    const utxo = await getUtxoOfAddressAtHeight(addresses, height);
    if (utxo.length > 0) {
      utxos = utxos.concat(utxo);
    }
  }
  return utxos;
};
