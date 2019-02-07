/**
 * Search an address having a specific txid
 * todo : Handle when multiples one (inbound/outbound)
 * @param txid
 * @return {{txid: *, address: null, type: null, found: boolean}}
 */
const searchAddressWithTx = function (txid) {
  const search = {
    txid,
    address: null,
    type: null,
    found: false,
  };
  const store = this.getStore();

  // Look up by looping over all addresses todo:optimisation
  const existingWallets = Object.keys(store);
  existingWallets.forEach((walletId) => {
    const existingTypes = Object.keys(store.wallets[walletId].addresses);
    existingTypes.forEach((type) => {
      const existingPaths = Object.keys(store.wallets[walletId].addresses[type]);
      existingPaths.forEach((path) => {
        const el = store.wallets[walletId].addresses[type][path];
        if (el.transactions.includes(search.txid)) {
          search.path = path;
          search.address = el.address;
          search.type = type;
          search.found = true;
          search.result = el;
        }
      });
    });
  });

  return search;
};
module.exports = searchAddressWithTx;
