import DAPIClient from '@dashevo/dapi-client';
import { Identifier } from '@dashevo/wasm-dpp';

type NonceState = {
  value: number,
  lastFetchedAt: number,
};

// 20 min
export const NONCE_FETCH_INTERVAL = 1200 * 1000;

class NonceManager {
  public dapiClient: DAPIClient;

  private identityNonce: Map<Identifier, NonceState>;

  private identityContractNonce: Map<Identifier, Map<Identifier, NonceState>>;

  constructor(dapiClient: DAPIClient) {
    this.dapiClient = dapiClient;

    this.identityNonce = new Map();
    this.identityContractNonce = new Map();
  }

  public setIdentityNonce(identityId: Identifier, nonce: number) {
    const nonceState = this.identityNonce.get(identityId);

    if (!nonceState) {
      this.identityNonce.set(identityId, {
        value: nonce,
        lastFetchedAt: Date.now(),
      });
    } else {
      nonceState.value = nonce;
    }
  }

  public async getIdentityNonce(identityId: Identifier): Promise<number> {
    let nonceState = this.identityNonce.get(identityId);

    if (typeof nonceState === 'undefined') {
      const { identityNonce } = await this.dapiClient.platform.getIdentityNonce(identityId);

      if (typeof identityNonce === 'undefined') {
        throw new Error('Identity nonce is not found');
      }

      nonceState = {
        value: identityNonce,
        lastFetchedAt: Date.now(),
      };

      this.identityNonce.set(identityId, nonceState);
    } else {
      const now = Date.now();
      if (now - nonceState.lastFetchedAt > NONCE_FETCH_INTERVAL) {
        const { identityNonce } = await this.dapiClient.platform.getIdentityNonce(identityId);

        if (typeof identityNonce === 'undefined') {
          throw new Error('Identity nonce is not found');
        }

        nonceState.value = identityNonce;
        nonceState.lastFetchedAt = now;
      }
    }

    return nonceState.value;
  }

  public setIdentityContractNonce(identityId: Identifier, contractId: Identifier, nonce: number) {
    let contractNonce = this.identityContractNonce.get(identityId);

    if (!contractNonce) {
      contractNonce = new Map();
      this.identityContractNonce.set(identityId, contractNonce);
    }

    const nonceState = contractNonce.get(contractId);

    if (!nonceState) {
      contractNonce.set(contractId, {
        value: nonce,
        lastFetchedAt: Date.now(),
      });
    } else {
      nonceState.value = nonce;
    }
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

    let nonceState = contractNonce.get(contractId);

    if (typeof nonceState === 'undefined') {
      const { identityContractNonce } = await this.dapiClient.platform
        .getIdentityContractNonce(identityId, contractId);

      if (typeof identityContractNonce === 'undefined') {
        throw new Error('Identity contract nonce is not found');
      }

      nonceState = {
        value: identityContractNonce,
        lastFetchedAt: Date.now(),
      };

      contractNonce.set(identityId, nonceState);
    } else {
      const now = Date.now();
      if (now - nonceState.lastFetchedAt > NONCE_FETCH_INTERVAL) {
        const { identityNonceContract } = await this.dapiClient.platform
          .getIdentityContractNonce(identityId, contractId);

        if (typeof identityNonceContract === 'undefined') {
          throw new Error('Identity nonce is not found');
        }

        nonceState.value = identityNonceContract;
        nonceState.lastFetchedAt = now;
      }
    }

    return nonceState.value;
  }
}

export default NonceManager;
