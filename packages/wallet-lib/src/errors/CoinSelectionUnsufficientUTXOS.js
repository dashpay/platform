const WalletLibError = require('./WalletLibError');

class CoinSelectionUnsufficientUTXOS extends WalletLibError {
  constructor(info) {
    const getErrorMessageOf = (_info) => {
      const { utxosValue, outputValue } = _info;
      const diff = utxosValue - outputValue;
      return `Unsufficient utxos (${utxosValue}) to cover the output : ${outputValue}. Diff : ${diff}`;
    };
    super(getErrorMessageOf(info));
  }
}
module.exports = CoinSelectionUnsufficientUTXOS;
