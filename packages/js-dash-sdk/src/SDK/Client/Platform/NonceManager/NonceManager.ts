import DAPIClient from '@dashevo/dapi-client';
import { Identifier } from '@dashevo/wasm-dpp';

class NonceManager {
  public dapiClient: DAPIClient;

  private identityNonce: Map<Identifier, number>;

  private identityContractNonce: Map<Identifier, Map<Identifier, number>>;

  constructor(dapiClient: DAPIClient) {
    this.dapiClient = dapiClient;

    this.identityNonce = new Map();
    this.identityContractNonce = new Map();
  }

  public setIdentityNonce(identityId: Identifier, nonce: number) {
    this.identityNonce.set(identityId, nonce);
  }

  public async getIdentityNonce(identityId: Identifier): Promise<number> {
    let nonce = this.identityNonce.get(identityId);

    if (typeof nonce === 'undefined') {
      ({ identityNonce: nonce } = await this.dapiClient.platform.getIdentityNonce(identityId));

      if (typeof nonce === 'undefined') {
        throw new Error('Identity nonce is not found');
      }

      this.identityNonce.set(identityId, nonce);
    }

    return nonce;
  }

  public setIdentityContractNonce(identityId: Identifier, contractId: Identifier, nonce: number) {
    let contractNonce = this.identityContractNonce.get(identityId);

    if (!contractNonce) {
      contractNonce = new Map();
      this.identityContractNonce.set(identityId, contractNonce);
    }

    contractNonce.set(contractId, nonce);
  }

  public async getIdentityContractNonce(
    identityId: Identifier,
    contractId: Identifier,
  ): Promise<number> {
    let contractNonce = this.identityContractNonce.get(identityId);

    if (!contractNonce) {
      contractNonce = new Map();
      this.identityContractNonce.set(identityId, contractNonce);
    }

    let nonce = contractNonce.get(contractId);

    if (typeof nonce === 'undefined') {
      ({ identityContractNonce: nonce } = await this.dapiClient.platform
        .getIdentityContractNonce(identityId, contractId));

      if (typeof nonce === 'undefined') {
        throw new Error('Identity contract nonce is not found');
      }

      contractNonce.set(identityId, nonce);
    }

    return nonce;
  }
}

export default NonceManager;
