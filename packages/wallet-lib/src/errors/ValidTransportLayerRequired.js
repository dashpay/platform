import WalletLibError from './WalletLibError.js';

class ValidTransportLayerRequired extends WalletLibError {
  constructor(method) {
    super(`A transport layer is needed to perform a ${method}`);
  }
}
export default ValidTransportLayerRequired;