export interface DPNSName {
  readonly label: string;
  readonly normalizedLabel: string;
  readonly normalizedParentDomainName: string;
  readonly preorderSalt: Uint8Array;
  readonly records: DPNSRecord;
  readonly subdomainRules?: SubdomainRules;
  readonly ownerId: string;
  readonly dataContractId: string;
}

export interface DPNSRecord {
  readonly dashUniqueIdentityId?: string;
  readonly dashAliasIdentityId?: string;
}

export interface SubdomainRules {
  readonly allowSubdomains: boolean;
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