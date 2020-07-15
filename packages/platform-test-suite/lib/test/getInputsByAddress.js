/**
 *
 * @param {DAPIClient} dapiClient
 * @param {string} address
 * @return {Promise<[]>}
 */
module.exports = async function getInputsByAddress(dapiClient, address) {
  const { items: utxos } = await dapiClient.core.getUTXO(address);
  let inputs = [];

  if (process.env.NETWORK === 'regtest' || process.env.NETWORK === 'local') {
    const { blocks } = await dapiClient.core.getStatus();
    const sortedUtxos = utxos
      .filter((utxo) => utxo.height < blocks - 100)
      .sort((a, b) => a.satoshis > b.satoshis);

    let sum = 0;
    let i = 0;
    if (sortedUtxos.length > 0) {
      do {
        const input = sortedUtxos[i];
        inputs.push(input);
        sum += input.satoshis;
        ++i;
      } while (sum < 1 && i < sortedUtxos.length);
    }
  } else {
    inputs = utxos;
  }

  return inputs;
};
