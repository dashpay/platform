import WalletLibError from './WalletLibError.js';

class UnknownStrategy extends WalletLibError {
  constructor(strategyName) {
    const getErrorMessageOf = () => `Unknown Strategy : ${strategyName}.`;
    super(getErrorMessageOf());
  }
}

export default UnknownStrategy;