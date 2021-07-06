import DataContract from "@dashevo/dpp/lib/dataContract/DataContract";
import Identity from "@dashevo/dpp/lib/identity/Identity";
import Identifier from "@dashevo/dpp/lib/Identifier";
import Client from '../Client';

class StateRepository {
  private readonly client: Client;

  constructor(client: Client) {
    this.client = client;
  }

  async fetchIdentity(id: Identifier|string): Promise<Identity|null> {
    return this.client.platform.identities.get(id);
  }

  async fetchDataContract(identifier: Identifier|string): Promise<DataContract|null> {
    return this.client.platform.contracts.get(identifier);
  }

  async isAssetLockTransactionOutPointAlreadyUsed(): Promise<boolean> {
    // This check still exists on the client side, however there's no need to
    // perform the check as in this client we always use a new transaction
    // register/top up identity
    return false;
  }

  async verifyInstantLock(): Promise<boolean> {
    // verification will be implemented later with DAPI SPV functionality
    return true;
  }

  async fetchTransaction(id: string): Promise<{ data: Buffer, height: number }> {
    const walletAccount = await this.client.getWalletAccount();

    const transaction = await walletAccount.getTransaction(id);

    return {
      // @ts-ignore
      data: transaction.toBuffer(),
      // we don't have transaction heights atm and it will be implemented later with DAPI SPV functionality
      height: 1,
    };
  }

  async fetchLatestPlatformBlockHeader(id: string): Promise<{ coreChainLockedHeight: number }> {
    const coreChainLockedHeight = await this.client.wallet!.transport.getBestBlockHeight();

    return {
      coreChainLockedHeight,
    };
  }
}

export default StateRepository;