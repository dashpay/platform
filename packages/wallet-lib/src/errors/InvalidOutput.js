const { has } = require('lodash');
const is = require('../utils/is');

const WalletLibError = require('./WalletLibError');

class InvalidOutput extends WalletLibError {
  constructor(output) {
    const getErrorMessageOf = (utxoErrors) => {
      if (!is.arr(utxoErrors) || utxoErrors.length === 0) return false;
      const err = utxoErrors[0];
      const txid = (has(output, 'txid')) ? output.txid : 'unknown';
      const address = (has(output, 'address')) ? output.address : 'unknown';
      return `Output txid:${txid} address: ${address} should have property ${err[0]} of type ${err[1]}`;
    };

    const evaluateUTXOObjectError = (_utxo) => {
      const utxosErrors = [];
      const expectedProps = [
        ['address', 'string'],
        ['satoshis', 'num'],
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
    super(getErrorMessageOf(evaluateUTXOObjectError(output)));
  }
}
module.exports = InvalidOutput;
