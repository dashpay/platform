import DashPlatformProtocol from "@dashevo/dpp";
import DataContract from "@dashevo/dpp/lib/dataContract/DataContract";
import Identity from "@dashevo/dpp/lib/identity/Identity";
import Identifier from "@dashevo/dpp/lib/Identifier";
import Client from '../Client';

class StateRepository {
  private readonly client: Client;
  private readonly dpp: DashPlatformProtocol;

  constructor(client: Client, dpp: DashPlatformProtocol) {
    this.client = client;
    this.dpp = dpp;
  }

  async fetchIdentity(id: Identifier|string): Promise<Identity|null> {
    const identifier = Identifier.from(id);

    const identityBuffer = await this.client.getDAPIClient().platform.getIdentity(identifier);

    if (identityBuffer === null) {
      return null;
    }

    return this.dpp.identity.createFromBuffer(identityBuffer);
  }

  async fetchDataContract(identifier: Identifier|string): Promise<DataContract|null> {
    const contractId: Identifier = Identifier.from(identifier);

    // Try to get contract from the cache
    for (const appName of this.client.getApps().getNames()) {
      const appDefinition = this.client.getApps().get(appName);
      if (appDefinition.contractId.equals(contractId) && appDefinition.contract) {
        return appDefinition.contract;
      }
    }

    // Fetch contract otherwise
    const rawContract = await this.client.getDAPIClient().platform.getDataContract(contractId);

    if (!rawContract) {
      return null;
    }

    const contract = await this.dpp.dataContract.createFromBuffer(rawContract);

    // Store contract to the cache
    for (const appName of this.client.getApps().getNames()) {
      const appDefinition = this.client.getApps().get(appName);
      if (appDefinition.contractId.equals(contractId)) {
        appDefinition.contract = contract;
      }
    }

    return contract;
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