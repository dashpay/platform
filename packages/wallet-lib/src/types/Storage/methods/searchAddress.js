/**
 * Search a specific address in the store
 * @param address
 * @param forceLoop - boolean - default : false - When set at true, will force a full search
 * @return {{address: *, type: null, found: boolean}}
 */
const searchAddress = function (address, forceLoop = false) {
  const search = {
    address,
    type: null,
    found: false,
  };
  const { store } = this;
  if (forceLoop === true) {
    // Look up by looping over all addresses todo:optimisation
    const existingWallets = Object.keys(store.wallets);
    existingWallets.forEach((walletId) => {
      const existingTypes = Object.keys(store.wallets[walletId].addresses);
      existingTypes.forEach((type) => {
        const existingPaths = Object.keys(store.wallets[walletId].addresses[type]);
        existingPaths.forEach((path) => {
          const el = store.wallets[walletId].addresses[type][path];
          if (el.address === search.address) {
            search.path = path;
            search.type = type;
            search.found = true;
            search.result = el;
            search.walletId = walletId;
          }
        });
      });
    });
  } else if (this.mappedAddress[address]) {
    const { path, type, walletId } = this.mappedAddress[address];
    const el = store.wallets[walletId].addresses[type][path];

    search.path = path;
    search.type = type;
    search.found = true;
    search.result = el;
    search.walletId = walletId;
  }


  return search;
};
module.exports = searchAddress;
