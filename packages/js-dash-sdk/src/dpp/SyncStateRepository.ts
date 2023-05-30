import { DataContract, Identity, Identifier } from '@dashevo/wasm-dpp';
import ValidationData from './ValidationData';

class SyncStateRepository {
  validationData: ValidationData;

  constructor(basicValidationData: ValidationData) {
    this.validationData = basicValidationData;
  }

  fetchIdentity(id: Identifier | string): Identity | null {
    return this.validationData.getIdentity(id);
  }

  fetchDataContract(identifier: Identifier | string): DataContract | null {
    return this.validationData.getDataContract(identifier);
  }

  // eslint-disable-next-line
  isAssetLockTransactionOutPointAlreadyUsed(): boolean {
    // This check still exists on the client side, however there's no need to
    // perform the check as in this client we always use a new transaction
    // register/top up identity
    return false;
  }

  // eslint-disable-next-line
  verifyInstantLock(): boolean {
    // verification will be implemented later with DAPI SPV functionality
    return true;
  }

  fetchTransaction(id: string): { data: Buffer, height: number } | null {
    return this.validationData.getTransaction(id);
  }

  fetchLatestPlatformCoreChainLockedHeight(): number {
    return this.validationData.getLatestPlatformCoreChainLockedHeight();
  }
}

export default SyncStateRepository;
