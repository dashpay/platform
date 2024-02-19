import { Identifier } from '@dashevo/wasm-dpp';

class NonceManager {
  private identityNonce: Map<Identifier, number>;

  private identityContractNonce: Map<Identifier, Map<Identifier, number>>;

  constructor() {
    this.identityNonce = new Map();
    this.identityContractNonce = new Map();
  }

  public incrementIdentityNonce(identityId: Identifier) {
    const nonce = this.identityNonce.get(identityId);

    if (typeof nonce === 'undefined') {
      this.identityNonce.set(identityId, 1);
    } else {
      this.identityNonce.set(identityId, nonce + 1);
    }
  }

  public setIdentityNonce(identityId: Identifier, nonce: number) {
    this.identityNonce.set(identityId, nonce);
  }

  public getIdentityNonce(identityId: Identifier): number | undefined {
    return this.identityNonce.get(identityId);
  }

  public incrementIdentityContractNonce(identityId: Identifier, contractId: Identifier) {
    let contractNonce = this.identityContractNonce.get(identityId);

    if (!contractNonce) {
      contractNonce = new Map();
      this.identityContractNonce.set(identityId, contractNonce);
    }

    const nonce = contractNonce.get(contractId);

    if (typeof nonce === 'undefined') {
      contractNonce.set(contractId, 1);
    } else {
      contractNonce.set(contractId, nonce + 1);
    }
  }

  public setIdentityContractNonce(identityId: Identifier, contractId: Identifier, nonce: number) {
    let contractNonce = this.identityContractNonce.get(identityId);

    if (!contractNonce) {
      contractNonce = new Map();
      this.identityContractNonce.set(identityId, contractNonce);
    }

    contractNonce.set(contractId, nonce);
  }

  public getIdentityContractNonce(
    identityId: Identifier,
    contractId: Identifier,
  ): number | undefined {
    return this.identityContractNonce.get(identityId)?.get(contractId);
  }
}

export default NonceManager;
