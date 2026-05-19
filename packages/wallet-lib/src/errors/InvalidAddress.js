import WalletLibError from './WalletLibError.js';

class InvalidAddress extends WalletLibError {
  constructor(address) {
    super(`Address Invalid : ${address} `);
  }
}
export default InvalidAddress;