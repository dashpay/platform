const WalletLibError = require('./WalletLibError');

class InvalidAddress extends WalletLibError {
  constructor(address) {
    const getErrorMessageOf = () => `Address Invalid : ${address} `;

    super(getErrorMessageOf((address)));
  }
}
module.exports = InvalidAddress;
