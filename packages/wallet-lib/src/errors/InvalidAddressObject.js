const is = require('../utils/is');
const WalletLibError = require('./WalletLibError');

class InvalidAddressObject extends WalletLibError {
  constructor(addressObject) {
    const getErrorMessageOf = (addressErrors) => {
      if (!is.arr(addressErrors) || addressErrors.length === 0) return false;
      const err = addressErrors[0];
      return `Address should have property ${err[0]} of type ${err[1]}`;
    };

    const evaluateAddressObjectError = (addrObj) => {
      const addressErrors = [];
      const expectedProps = [
        ['path', 'string'],
        ['address', 'addressObject'],
      ];
      const handledTypeVerification = Object.keys(is);
      expectedProps.forEach((prop) => {
        const key = prop[0];
        const type = prop[1];
        if (handledTypeVerification.includes(type)) {
          if (!is[type](addrObj[key])) {
            addressErrors.push(prop);
          }
        }
      });
      return addressErrors;
    };
    super(getErrorMessageOf(evaluateAddressObjectError(addressObject)));
  }
}
module.exports = InvalidAddressObject;
