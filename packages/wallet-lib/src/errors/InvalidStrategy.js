import WalletLibError from './WalletLibError.js';

class InvalidStrategy extends WalletLibError {
  constructor(arg) {
    const type = arg.constructor.name;
    super(`Unable to import strategy. Expected 'str' or 'fn' got ${type}`);
  }
}
export default InvalidStrategy;