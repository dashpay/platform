const WalletLibError = require('./WalletLibError');

class InvalidAddress extends WalletLibError {
  constructor(address) {
    super(`Address Invalid : ${address} `);
  }
}
module.exports = InvalidAddress;
