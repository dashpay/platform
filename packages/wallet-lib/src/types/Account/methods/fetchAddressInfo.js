const logger = require('../../../logger');
const { ValidTransportLayerRequired } = require('../../../errors');
const { is } = require('../../../utils');

/**
 * Fetch a specific address from the transport layer
 * @param addressObj - AddressObject having an address and a path
 * @param fetchUtxo - If we also query the utxo (default: yes)
 * @return {Promise<addrInfo>}
 */
async function fetchAddressInfo(addressObj, fetchUtxo = true) {
  if (!this.transporter.isValid) throw new ValidTransportLayerRequired('fetchAddressInfo');
  const self = this;
  const { address, path, index } = addressObj;

  try {
    const addrSum = await this.transporter.getAddressSummary(address);
    if (!addrSum) return false;
    const {
      balanceSat, unconfirmedBalanceSat, transactions,
    } = addrSum;

    if (is.undef(balanceSat)
        || is.undef(unconfirmedBalanceSat)
        || !is.arr(transactions)) {
      return false;
    }

    const addrInfo = {
      address,
      path,
      index,
      balanceSat,
      unconfirmedBalanceSat,
      transactions,
      fetchedLast: +new Date(),
    };
    addrInfo.used = (transactions.length > 0);

    if (transactions.length > 0) {
      // If we have cacheTx, then we will check if we know this transactions
      if (self.cacheTx) {
        const promises = [];
        transactions.forEach((txid) => {
          const knownTx = Object.keys(self.store.transactions);
          if (!knownTx.includes(txid)) {
            const promise = self.getTransaction(txid);
            promises.push(promise);
          }
        });
        await Promise.all(promises);
      }
    }
    if (fetchUtxo) {
      const fetchedUtxoReq = await self.transporter.getUTXO(address);
      if (fetchedUtxoReq && fetchedUtxoReq.totalItems) {
        const fetchedUtxo = fetchedUtxoReq.items;

        const utxos = [];
        if (balanceSat > 0) {
          fetchedUtxo.forEach((utxo) => {
            utxos.push({
              satoshis: utxo.satoshis,
              txid: utxo.txid,
              address: utxo.address,
              outputIndex: utxo.outputIndex,
              scriptPubKey: utxo.script,
              // scriptSig: utxo.scriptSig,
            });
          });
        }
        if (utxos.length > 0) {
          self.storage.addUTXOToAddress(utxos, addressObj.address);
        }
      }
    }
    return addrInfo;
  } catch (err) {
    logger.error('Error', err);
    return false;
  }
}

module.exports = fetchAddressInfo;
