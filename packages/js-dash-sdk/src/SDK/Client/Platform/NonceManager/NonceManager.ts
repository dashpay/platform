import DAPIClient from '@dashevo/dapi-client';
import { Identifier } from '@dashevo/wasm-dpp';

type NonceState = {
  value: bigint,
  lastFetchedAt: number,
};

// 20 min
export const NONCE_FETCH_INTERVAL = 1200 * 1000;

class NonceManager {
  public dapiClient: DAPIClient;

  private identityNonce: Map<string, NonceState>;

  private identityContractNonce: Map<string, Map<string, NonceState>>;

  constructor(dapiClient: DAPIClient) {
    this.dapiClient = dapiClient;

    this.identityNonce = new Map();
    this.identityContractNonce = new Map();
  }

  public setIdentityNonce(identityId: Identifier, nonce: bigint) {
    const identityIdStr = identityId.toString();
    const nonceState = this.identityNonce.get(identityIdStr);

    if (!nonceState) {
      this.identityNonce.set(identityIdStr, {
        value: nonce,
        lastFetchedAt: Date.now(),
      });
    } else {
      nonceState.value = nonce;
    }
  }

  public async getIdentityNonce(identityId: Identifier): Promise<bigint> {
    const identityIdStr = identityId.toString();
    let nonceState = this.identityNonce.get(identityIdStr);

    if (typeof nonceState === 'undefined') {
      const { identityNonce } = await this.dapiClient.platform.getIdentityNonce(identityId);

      if (typeof identityNonce === 'undefined') {
        throw new Error('Identity nonce is not found');
      }

      nonceState = {
        value: identityNonce,
        lastFetchedAt: Date.now(),
      };

      this.identityNonce.set(identityIdStr, nonceState);
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

  public async bumpIdentityNonce(identityId: Identifier): Promise<bigint> {
    const identityNonce = await this.getIdentityNonce(identityId);
    const nextIdentityNonce = identityNonce + BigInt(1);

    this.setIdentityNonce(identityId, nextIdentityNonce);

    return nextIdentityNonce;
  }

  public setIdentityContractNonce(identityId: Identifier, contractId: Identifier, nonce: bigint) {
    const identityIdStr = identityId.toString();
    const contractIdStr = contractId.toString();

    let contractNonce = this.identityContractNonce.get(identityIdStr);

    if (!contractNonce) {
      contractNonce = new Map();
      this.identityContractNonce.set(identityIdStr, contractNonce);
    }

    const nonceState = contractNonce.get(contractIdStr);

    if (!nonceState) {
      contractNonce.set(contractIdStr, {
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
  ): Promise<bigint> {
    const identityIdStr = identityId.toString();
    const contractIdStr = contractId.toString();

    let contractNonce = this.identityContractNonce.get(identityIdStr);

    if (!contractNonce) {
      contractNonce = new Map();
      this.identityContractNonce.set(identityIdStr, contractNonce);
    }

    let nonceState = contractNonce.get(contractIdStr);

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

      contractNonce.set(contractIdStr, nonceState);
    } else {
      const now = Date.now();
      if (now - nonceState.lastFetchedAt > NONCE_FETCH_INTERVAL) {
        const { identityContractNonce } = await this.dapiClient.platform
          .getIdentityContractNonce(identityId, contractId);

        if (typeof identityContractNonce === 'undefined') {
          throw new Error('Identity nonce is not found');
        }

        nonceState.value = identityContractNonce;
        nonceState.lastFetchedAt = now;
      }
    }

    return nonceState.value;
  }

  public async bumpIdentityContractNonce(
    identityId: Identifier,
    contractId: Identifier,
  ): Promise<bigint> {
    const identityContractNonce = await this.getIdentityContractNonce(identityId, contractId);
    // @ts-ignore
    const nextIdentityContractNonce = identityContractNonce + BigInt(1);
    this.setIdentityContractNonce(identityId, contractId, nextIdentityContractNonce);
    return nextIdentityContractNonce;
  }
}

export default NonceManager;
