const { cloneDeep, xor } = require('lodash');
const logger = require('../../../logger');
const { InvalidAddressObject, TransactionNotInStore } = require('../../../errors');
const { is } = require('../../../utils');
const EVENTS = require('../../../EVENTS');

/**
* Update a specific address information in the store
* @param addressObj
* @param walletId
* @return {boolean}
*/
const updateAddress = function (addressObj, walletId) {
  if (!walletId) throw new Error('Expected walletId to update an address');
  if (!is.addressObj(addressObj)) throw new InvalidAddressObject(addressObj);
  const { path } = addressObj;
  if (!path) throw new Error('Expected path to update an address');
  const accountIndex = parseInt(path.split('/')[3], 10);

  const typeInt = path.split('/')[4];
  let type;
  switch (typeInt) {
    case '0':
      type = 'external';
      break;
    case '1':
      type = 'internal';
      break;
    default:
      type = 'misc';
  }
  const walletStore = this.store.wallets[walletId];
  const addressesStore = walletStore.addresses;
  const previousObject = cloneDeep(addressesStore[type][path]);

  const newObject = cloneDeep(addressObj);
  // We do not autorize to alter UTXO using this
  // if(newObject.utxos.length==0 && previousObject.utxos.length>0){
  //
  // }
  const currentBlockHeight = this.store.chains[walletStore.network].blockheight;

  // We calculate here the balanceSat and unconfirmedBalanceSat of our addressObj
  // We do that to avoid getBalance to be slow, so we have to keep that in mind or then
  // Move that to an event type of calculation or somth
  const { utxos } = newObject;

  const newObjectUtxosKeys = Object.keys(utxos);
  if (newObjectUtxosKeys.length > 0) {
    // we compare the diff between the two utxos sets

    const previousUTXOS = (previousObject !== undefined) ? previousObject.utxos : [];

    const newUtxos = xor(newObjectUtxosKeys, Object.keys(previousUTXOS));
    // Then we verify the outputs

    newUtxos.forEach((utxoKey) => {
      const utxo = utxos[utxoKey];
      try {
        const { blockheight } = this.getTransaction(utxo.txid);
        if (currentBlockHeight - blockheight >= 6) newObject.balanceSat += utxo.satoshis;
        else newObject.unconfirmedBalanceSat += utxo.satoshis;
      } catch (e) {
        logger.error('Error', e);
        if (e instanceof TransactionNotInStore) {
          // TODO : We consider unconfirmed a transaction that we don't know of, should we ?
          newObject.unconfirmedBalanceSat += utxo.satoshis;
        } else throw e;
      }
    });
  }


  // Check if there is a balance but no utxo.
  addressesStore[type][path] = newObject;
  if (previousObject === undefined) {
    if (newObject.balanceSat > 0) {
      this.announce(
        EVENTS.CONFIRMED_BALANCE_CHANGED,
        {
          delta: newObject.balanceSat,
          currentValue: this.calculateDuffBalance(walletId, accountIndex, 'confirmed') || newObject.unconfirmedBalanceSat,
          // currentValue: newObject.balanceSat,
        },
      );
    }
    if (newObject.unconfirmedBalanceSat > 0) {
      this.announce(
        EVENTS.UNCONFIRMED_BALANCE_CHANGED,
        {
          delta: newObject.unconfirmedBalanceSat,
          // currentValue: newObject.unconfirmedBalanceSat,
          currentValue: this.calculateDuffBalance(walletId, accountIndex, 'unconfirmed'),
        },
      );
    }
  } else {
    if (previousObject.balanceSat !== newObject.balanceSat) {
      this.announce(
        EVENTS.CONFIRMED_BALANCE_CHANGED,
        {
          delta: newObject.balanceSat - previousObject.balanceSat,
          // currentValue: newObject.balanceSat,
          currentValue: this.calculateDuffBalance(walletId, accountIndex, 'confirmed'),
        },
      );
    }
    if (previousObject.unconfirmedBalanceSat !== newObject.unconfirmedBalanceSat) {
      this.announce(EVENTS.UNCONFIRMED_BALANCE_CHANGED,
        {
          delta: newObject.unconfirmedBalanceSat - previousObject.unconfirmedBalanceSat,
          // currentValue: newObject.unconfirmedBalanceSat,
          currentValue: this.calculateDuffBalance(walletId, accountIndex, 'unconfirmed'),
        });
    }
  }

  this.lastModified = Date.now();

  if (!this.mappedAddress[newObject.address]) {
    this.mappedAddress[newObject.address] = { walletId, type, path };
  }
  return true;
};
module.exports = updateAddress;
