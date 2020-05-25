const { is } = require('../../../../utils');
const logger = require('../../../../logger');

module.exports = async function getUTXO(address) {
  logger.silly(`DAPIClient.getUTXO[${address}]`);
  if (!is.address(address) && !is.arr(address)) throw new Error('Received an invalid address to fetch');

  let from = 0;
  let to = 1000;
  let utxos = [];

  const fetchAndReturnUTXO = async (_from, _to) => {
    try {
      const req = await this.client.getUTXO(address, _from, _to);
      return { ...req, size: req.items.length };
    } catch (e) {
      throw new Error(`Error fetching UTXO ${address}:{_from}:{_to} - ${e.message}`);
    }
  };

  const firstRequest = await fetchAndReturnUTXO(from, to);
  utxos = utxos.concat(firstRequest.items);

  if (firstRequest.totalItems > firstRequest.to) {
    const promises = [];
    const numberOfPromises = Math.floor((firstRequest.totalItems - 1000) / 1000);

    for (let i = 0; i < numberOfPromises; i = +1) {
      from = to;
      to += 1000;
      promises.push(fetchAndReturnUTXO(from, to));
    }

    const rest = ((firstRequest.totalItems + 1 + numberOfPromises) % 1000);
    if (rest) {
      from = to;
      to += rest;
      promises.push(fetchAndReturnUTXO(from, to));
    }

    const resolvedPromises = await Promise.all(promises);
    resolvedPromises.forEach((res) => {
      if (res.items) {
        utxos = utxos.concat(res.items);
      }
    });
  }

  return utxos;
};
