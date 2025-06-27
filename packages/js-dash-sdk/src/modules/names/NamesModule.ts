import { SDK } from '../../SDK';
import { getWasmSdk } from '../../core/WasmLoader';
import { StateTransitionResult, ProofOptions } from '../../core/types';
import { 
  DPNSName, 
  DPNSRecord, 
  NameRegisterOptions,
  NameSearchOptions 
} from './types';

export class NamesModule {
  private readonly DPNS_CONTRACT_IDS = {
    mainnet: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
    testnet: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31iV'
  };
  
  constructor(private sdk: SDK) {}

  private ensureInitialized(): void {
    if (!this.sdk.isInitialized()) {
      throw new Error('SDK not initialized. Call SDK.initialize() first.');
    }
  }

  private getDPNSContractId(): string {
    // Check if DPNS app is registered
    const dpnsApp = this.sdk.getApp('dpns');
    if (dpnsApp) {
      return dpnsApp.contractId;
    }
    
    // Use default based on network
    const network = this.sdk.getNetwork();
    const contractId = this.DPNS_CONTRACT_IDS[network.type as 'mainnet' | 'testnet'];
    
    if (!contractId) {
      throw new Error(`DPNS contract ID not configured for network: ${network.type}`);
    }
    
    return contractId;
  }

  async register(options: NameRegisterOptions): Promise<StateTransitionResult> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    // Validate identity exists
    const identityModule = await import('../identities/IdentityModule');
    const identity = await new identityModule.IdentityModule(this.sdk).get(options.ownerId);
    
    if (!identity) {
      throw new Error(`Owner identity ${options.ownerId} not found`);
    }
    
    // Check if name is available
    const existingName = await this.resolve(options.label);
    if (existingName) {
      throw new Error(`Name '${options.label}' is already registered`);
    }
    
    // Normalize label
    const normalizedLabel = this.normalizeLabel(options.label);
    
    // Generate preorder salt
    const preorderSalt = crypto.getRandomValues(new Uint8Array(32));
    
    // Create DPNS document
    const dpnsDocument = {
      label: options.label,
      normalizedLabel,
      normalizedParentDomainName: 'dash',
      preorderSalt,
      records: options.records || {
        dashUniqueIdentityId: options.ownerId
      },
      subdomainRules: {
        allowSubdomains: false
      }
    };
    
    // Create document using documents module
    const documentsModule = await import('../documents/DocumentModule');
    const documents = new documentsModule.DocumentModule(this.sdk);
    
    const result = await documents.broadcast(
      this.getDPNSContractId(),
      options.ownerId,
      {
        create: [{
          type: 'domain',
          data: dpnsDocument
        }]
      }
    );
    
    return result;
  }

  async resolve(name: string, options: ProofOptions = {}): Promise<DPNSName | null> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    const normalizedLabel = this.normalizeLabel(name);
    
    // Query for the name
    const documentsModule = await import('../documents/DocumentModule');
    const documents = new documentsModule.DocumentModule(this.sdk);
    
    const results = await documents.query({
      dataContractId: this.getDPNSContractId(),
      type: 'domain',
      where: [
        ['=', 'normalizedLabel', normalizedLabel],
        ['=', 'normalizedParentDomainName', 'dash']
      ],
      limit: 1
    }, options);
    
    if (results.length === 0) {
      return null;
    }
    
    const doc = results[0];
    return {
      label: doc.data.label,
      normalizedLabel: doc.data.normalizedLabel,
      normalizedParentDomainName: doc.data.normalizedParentDomainName,
      preorderSalt: doc.data.preorderSalt,
      records: doc.data.records,
      subdomainRules: doc.data.subdomainRules,
      ownerId: doc.ownerId,
      dataContractId: doc.dataContractId
    };
  }

  async resolveByRecord(
    recordType: 'dashUniqueIdentityId' | 'dashAliasIdentityId',
    recordValue: string,
    options: ProofOptions = {}
  ): Promise<DPNSName[]> {
    this.ensureInitialized();
    
    const documentsModule = await import('../documents/DocumentModule');
    const documents = new documentsModule.DocumentModule(this.sdk);
    
    const results = await documents.query({
      dataContractId: this.getDPNSContractId(),
      type: 'domain',
      where: [
        ['=', `records.${recordType}`, recordValue]
      ],
      limit: 100
    }, options);
    
    return results.map(doc => ({
      label: doc.data.label,
      normalizedLabel: doc.data.normalizedLabel,
      normalizedParentDomainName: doc.data.normalizedParentDomainName,
      preorderSalt: doc.data.preorderSalt,
      records: doc.data.records,
      subdomainRules: doc.data.subdomainRules,
      ownerId: doc.ownerId,
      dataContractId: doc.dataContractId
    }));
  }

  async search(
    pattern: string,
    options: NameSearchOptions = {}
  ): Promise<DPNSName[]> {
    this.ensureInitialized();
    
    const documentsModule = await import('../documents/DocumentModule');
    const documents = new documentsModule.DocumentModule(this.sdk);
    
    const where: any[] = [];
    
    // Add pattern search
    if (pattern) {
      where.push(['startsWith', 'normalizedLabel', this.normalizeLabel(pattern)]);
    }
    
    // Add parent domain filter
    const parentDomain = options.parentDomain || 'dash';
    where.push(['=', 'normalizedParentDomainName', parentDomain]);
    
    const results = await documents.query({
      dataContractId: this.getDPNSContractId(),
      type: 'domain',
      where,
      limit: options.limit || 25,
      startAfter: options.startAfter,
      orderBy: [['normalizedLabel', 'asc']]
    });
    
    return results.map(doc => ({
      label: doc.data.label,
      normalizedLabel: doc.data.normalizedLabel,
      normalizedParentDomainName: doc.data.normalizedParentDomainName,
      preorderSalt: doc.data.preorderSalt,
      records: doc.data.records,
      subdomainRules: doc.data.subdomainRules,
      ownerId: doc.ownerId,
      dataContractId: doc.dataContractId
    }));
  }

  async update(
    name: string,
    ownerId: string,
    records: DPNSRecord
  ): Promise<StateTransitionResult> {
    this.ensureInitialized();
    
    // Resolve the name first
    const dpnsName = await this.resolve(name);
    if (!dpnsName) {
      throw new Error(`Name '${name}' not found`);
    }
    
    // Verify ownership
    if (dpnsName.ownerId !== ownerId) {
      throw new Error(`Identity ${ownerId} does not own name '${name}'`);
    }
    
    // Get the document
    const documentsModule = await import('../documents/DocumentModule');
    const documents = new documentsModule.DocumentModule(this.sdk);
    
    const nameDocuments = await documents.query({
      dataContractId: this.getDPNSContractId(),
      type: 'domain',
      where: [
        ['=', 'normalizedLabel', dpnsName.normalizedLabel],
        ['=', 'normalizedParentDomainName', dpnsName.normalizedParentDomainName]
      ],
      limit: 1
    });
    
    if (nameDocuments.length === 0) {
      throw new Error('Name document not found');
    }
    
    const doc = nameDocuments[0];
    
    // Update records
    const updatedData = {
      ...doc.data,
      records
    };
    
    // Broadcast update
    return documents.broadcast(
      this.getDPNSContractId(),
      ownerId,
      {
        replace: [{
          id: doc.id,
          type: 'domain',
          data: updatedData,
          revision: doc.revision + 1
        }]
      }
    );
  }

  private normalizeLabel(label: string): string {
    // Convert to lowercase and remove invalid characters
    return label.toLowerCase()
      .replace(/[^a-z0-9-]/g, '')
      .replace(/^-+|-+$/g, ''); // Remove leading/trailing hyphens
  }
}