export interface DPNSName {
  label: string;
  normalizedLabel: string;
  normalizedParentDomainName: string;
  preorderSalt: Uint8Array;
  records: DPNSRecord;
  subdomainRules?: SubdomainRules;
  ownerId: string;
  contractId: string;
}

export interface DPNSRecord {
  dashUniqueIdentityId?: string;
  dashAliasIdentityId?: string;
}

export interface SubdomainRules {
  allowSubdomains: boolean;
}

export interface NameRegisterOptions {
  label: string;
  ownerId: string;
  records?: DPNSRecord;
}

export interface NameSearchOptions {
  parentDomain?: string;
  limit?: number;
  startAfter?: string;
}