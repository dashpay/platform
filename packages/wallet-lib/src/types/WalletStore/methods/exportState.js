const { cloneDeep } = require('lodash');

function exportState() {
  const { walletId } = this;
  const { mnemonic, paths, identities } = this.state;

  return {
    walletId,
    state: {
      mnemonic,
      paths: cloneDeep(Object.fromEntries(paths)),
      identities: cloneDeep(Object.fromEntries(identities)),
    },
  };
}
module.exports = exportState;
