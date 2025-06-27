// Core exports
export { SDK } from './SDK';
export * from './core/types';
export { 
  AbstractContextProvider,
  CentralizedProvider,
  loadWasmSdk
} from './core';

// Module exports - these can be imported separately for tree-shaking
export type { 
  Identity,
  IdentityPublicKey,
  IdentityCreateOptions,
  IdentityTopUpOptions,
  IdentityUpdateOptions,
  AssetLockProof,
  CreditTransferOptions,
  CreditWithdrawalOptions
} from './modules/identities';

export type {
  DataContract,
  DocumentSchema,
  Index,
  ContractCreateOptions,
  ContractUpdateOptions,
  ContractHistoryEntry,
  ContractVersion
} from './modules/contracts';

export type {
  Document,
  DocumentCreateOptions,
  DocumentReplaceOptions,
  DocumentDeleteOptions,
  DocumentsBatchOptions,
  DocumentQuery
} from './modules/documents';

export type {
  DPNSName,
  DPNSRecord,
  SubdomainRules,
  NameRegisterOptions,
  NameSearchOptions
} from './modules/names';

// Factory function for creating SDK with all modules
export function createSDK(options?: SDKOptions): DashSDK {
  return new DashSDK(options);
}

// Extended SDK class with all modules pre-loaded
import { SDK } from './SDK';
import { SDKOptions } from './core/types';
import { IdentityModule } from './modules/identities/IdentityModule';
import { ContractModule } from './modules/contracts/ContractModule';
import { DocumentModule } from './modules/documents/DocumentModule';
import { NamesModule } from './modules/names/NamesModule';

export class DashSDK extends SDK {
  public readonly identities: IdentityModule;
  public readonly contracts: ContractModule;
  public readonly documents: DocumentModule;
  public readonly names: NamesModule;
  
  constructor(options?: SDKOptions) {
    super(options);
    
    // Initialize modules
    this.identities = new IdentityModule(this);
    this.contracts = new ContractModule(this);
    this.documents = new DocumentModule(this);
    this.names = new NamesModule(this);
  }
}

// Bluetooth exports
export {
  BluetoothConnection,
  BluetoothProvider,
  BluetoothWallet,
  setupBluetoothSDK,
  createBluetoothSDK
} from './bluetooth';

// Default export
export default DashSDK;