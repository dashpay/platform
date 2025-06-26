import { DataContract, Identity, Identifier } from '@dashevo/wasm-dpp';
import Client from '../Client';

class StateRepository {
  private readonly client: Client;

  constructor(client: Client) {
    this.client = client;
  }

  async fetchIdentity(id: Identifier | string): Promise<Identity | null> {
    return this.client.platform.identities.get(id);
  }

  async fetchDataContract(identifier: Identifier | string): Promise<DataContract | null> {
    return this.client.platform.contracts.get(identifier);
  }

  // eslint-disable-next-line
  async isAssetLockTransactionOutPointAlreadyUsed(): Promise<boolean> {
    // This check still exists on the client side, however there's no need to
    // perform the check as in this client we always use a new transaction
    // register/top up identity
    return false;
  }

  // eslint-disable-next-line
  async verifyInstantLock(): Promise<boolean> {
    // verification will be implemented later with DAPI SPV functionality
    return true;
  }

  async fetchTransaction(id: string): Promise<{ data: Buffer, height: number }> {
    const walletAccount = await this.client.getWalletAccount();
    // @ts-ignore
    const { transaction } = await walletAccount.getTransaction(id);

    return {
      data: transaction.toBuffer(),
      // we don't have transaction heights atm
      // and it will be implemented later with DAPI SPV functionality
      height: 1,
    };
  }

  async fetchLatestPlatformCoreChainLockedHeight(): Promise<number> {
    return this.client.wallet!.transport.getBestBlockHeight();
  }
}

export default StateRepository;
