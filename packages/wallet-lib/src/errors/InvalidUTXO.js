const { has } = require('lodash');
const WalletLibError = require('./WalletLibError');
const is = require('../utils/is');

class InvalidUTXO extends WalletLibError {
  constructor(utxo) {
    const getErrorMessageOf = (utxoErrors) => {
      if (!is.arr(utxoErrors) || utxoErrors.length === 0) return false;
      const err = utxoErrors[0];
      const txid = (has(utxo, 'txid')) ? utxo.txid : 'unknown';
      return `UTXO txid:${txid} should have property ${err[0]} of type ${err[1]}; ${JSON.stringify(utxo)}`;
    };

    const evaluateUTXOObjectError = (_utxo) => {
      const utxosErrors = [];
      const expectedProps = [
        ['txid', 'txid'],
        ['outputIndex', 'num'],
        ['satoshis', 'num'],
        ['scriptPubKey', 'string'],
      ];
      const handledTypeVerification = Object.keys(is);
      expectedProps.forEach((prop) => {
        const key = prop[0];
        const type = prop[1];
        if (handledTypeVerification.includes(type)) {
          if (!is[type](_utxo[key])) {
            utxosErrors.push(prop);
          }
        }
      });
      return utxosErrors;
    };
    super(getErrorMessageOf(evaluateUTXOObjectError(utxo)));
  }
}
module.exports = InvalidUTXO;
