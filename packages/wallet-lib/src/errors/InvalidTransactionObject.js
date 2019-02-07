const { has } = require('lodash');
const is = require('../utils/is');

const WalletLibError = require('./WalletLibError');


class InvalidTransactionObject extends WalletLibError {
  constructor(transactionObj) {
    const getErrorMessageOf = (transactionErrors) => {
      if (!is.arr(transactionErrors) || transactionErrors.length === 0) return false;
      const err = transactionErrors[0];
      const txid = has(transactionObj, 'txid') ? transactionObj.txid : 'unknown';
      return `Transaction txid: ${txid} should have property ${err[0]} of type ${err[1]}`;
    };

    const evaluateTransactionObjectError = (_txObj) => {
      const addressErrors = [];
      const expectedProps = [
        ['txid', 'txid'],
        ['vin', 'array'],
        ['vout', 'array'],
      ];
      const handledTypeVerification = Object.keys(is);
      expectedProps.forEach((prop) => {
        const key = prop[0];
        const type = prop[1];
        if (handledTypeVerification.includes(type)) {
          if ((!has(_txObj, key) || !is[type](_txObj[key]))) {
            addressErrors.push(prop);
          }
        }
      });
      return addressErrors;
    };
    super(getErrorMessageOf(evaluateTransactionObjectError(transactionObj)));
  }
}
module.exports = InvalidTransactionObject;
