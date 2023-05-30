import {
  DataContract,
  DataContractCreateTransition, DataContractUpdateTransition, DocumentsBatchTransition,
  Identifier,
  Identity,
  IdentityCreateTransition, IdentityTopUpTransition, IdentityUpdateTransition,
  StateTransitionTypes,
} from '@dashevo/wasm-dpp';

import { Transaction, errors } from '@dashevo/dashcore-lib';

import Client from '../SDK/Client/Client';

type StateTransitionLike = DataContractCreateTransition
| DataContractUpdateTransition
| IdentityCreateTransition
| IdentityTopUpTransition
| IdentityUpdateTransition
| DocumentsBatchTransition;

class ValidationData {
  private identities = new Map<string, Identity | null>();

  private dataContracts = new Map<string, DataContract | null>();

  private transactions = new Map<string, { data: Buffer, height: number }>();

  private latestPlatformCoreChainLockedHeight: number | undefined;

  private readonly client: Client;

  constructor(client: Client) {
    this.client = client;
  }

  setIdentity(id: Identifier | string, identity: Identity | null) {
    this.identities.set(id.toString(), identity);
  }

  getIdentity(id: Identifier | string): Identity | null {
    const identity = this.identities.get(id.toString());

    if (identity === undefined) {
      throw new Error(`Identity ${id} is not fetched`);
    }

    return identity;
  }

  setDataContract(id: Identifier | string, dataContract: DataContract | null) {
    this.dataContracts.set(id.toString(), dataContract);
  }

  getDataContract(id: Identifier | string): DataContract | null {
    const dataContract = this.dataContracts.get(id.toString());

    if (dataContract === undefined) {
      throw new Error(`Data Contract ${id} is not fetched`);
    }

    return dataContract;
  }

  setTransaction(id: string, transaction: { data: Buffer, height: number }) {
    this.transactions.set(id, transaction);
  }

  getTransaction(id: string): { data: Buffer, height: number } | null {
    const transaction = this.transactions.get(id);

    if (transaction === undefined) {
      throw new Error(`Transaction ${id} is not fetched`);
    }

    return transaction;
  }

  getLatestPlatformCoreChainLockedHeight(): number {
    if (this.latestPlatformCoreChainLockedHeight === undefined) {
      throw new Error('Latest platform core chain locked height is not fetched');
    }

    return this.latestPlatformCoreChainLockedHeight;
  }

  setLatestPlatformCoreChainLockedHeight(height: number) {
    this.latestPlatformCoreChainLockedHeight = height;
  }

  async fetchForStateTransition(stateTransition: StateTransitionLike): Promise<void> {
    return this.fetchForRawStateTransition(stateTransition.toObject(true));
  }

  async fetchForRawStateTransition(rawStateTransition: Object) {
    // @ts-ignore
    switch (rawStateTransition.type) {
      case StateTransitionTypes.DocumentsBatch: {
        // Fetch unique data contracts from document transitions

        // @ts-ignore
        const dataContractIdHexes = (rawStateTransition.transitions || [])
          // @ts-ignore
          .map((transition) => transition.$dataContractId)
          .filter((id) => id !== undefined)
          .map((id) => id.toString('hex'));

        const uniqueDataContractIdHexes: Set<string> = new Set(dataContractIdHexes);

        const uniqueDataContractIds = Array.from(uniqueDataContractIdHexes)
          .map((idHex) => {
            const idBuffer = Buffer.from(idHex, 'hex');
            return Identifier.from(idBuffer);
          });

        const dataContracts = await Promise.all(
          uniqueDataContractIds.map((id) => this.client.platform.contracts.get(id)),
        );

        dataContracts.forEach((dataContract) => {
          this.setDataContract(dataContract.getId(), dataContract);
        });

        break;
      }
      case StateTransitionTypes.IdentityCreate:
      case StateTransitionTypes.IdentityTopUp: {
        // @ts-ignore
        switch (rawStateTransition?.assetLockProof?.type) {
          // Instant Asset Lock Proof
          case 0: {
            break;
          }
          // Chain Asset Lock Proof
          case 1: {
            // Fetch latest platform block height

            this.setLatestPlatformCoreChainLockedHeight(
              await this.client.wallet!.transport.getBestBlockHeight(),
            );

            // Fetch Asset Lock transactions

            // @ts-ignore
            const outPoint = rawStateTransition?.assetLockProof?.outPoint;

            if (outPoint && outPoint.length > 0) {
              let parsedOutPointBuffer;

              try {
                parsedOutPointBuffer = Transaction.parseOutPointBuffer(outPoint);
              } catch (e) {
                if (!(e instanceof errors.WrongOutPointError)) {
                  throw e;
                }
              }

              if (parsedOutPointBuffer) {
                const { transactionHash } = parsedOutPointBuffer;

                const walletAccount = await this.client.getWalletAccount();

                const getTransactionResult = await walletAccount.getTransaction(transactionHash);

                if (getTransactionResult) {
                  // @ts-ignore
                  const { transaction } = getTransactionResult;

                  this.setTransaction(transactionHash, {
                    data: transaction.toBuffer(),
                    // we don't have transaction heights atm
                    // and it will be implemented later with DAPI SPV functionality
                    height: 1,
                  });
                }
              }
            }

            break;
          }
          default: {
            throw new Error('Unknown asset lock proof type');
          }
        }
        break;
      }
      default:
    }
  }

  clear() {
    this.dataContracts.clear();
    this.identities.clear();
    this.transactions.clear();
    this.latestPlatformCoreChainLockedHeight = 0;
  }
}

export default ValidationData;
