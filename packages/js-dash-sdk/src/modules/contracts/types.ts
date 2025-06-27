export interface DataContract {
  id: string;
  ownerId: string;
  schema: object;
  version: number;
  documentSchemas: Record<string, DocumentSchema>;
}

export interface DocumentSchema {
  type: 'object';
  properties: Record<string, any>;
  required?: string[];
  additionalProperties?: boolean;
  indices?: Index[];
}

export interface Index {
  name: string;
  properties: Array<{
    [key: string]: 'asc' | 'desc';
  }>;
  unique?: boolean;
}

export interface ContractCreateOptions {
  ownerId: string;
  schema: object;
  documentSchemas: Record<string, DocumentSchema>;
}

export interface ContractUpdateOptions {
  schema?: object;
  documentSchemas?: Record<string, DocumentSchema>;
}

export interface ContractHistoryEntry {
  contractId: string;
  version: number;
  operation: 'create' | 'update';
  timestamp: number;
  changes: string[];
  transactionHash?: string;
}

export interface ContractVersion {
  version: number;
  schemaHash: string;
  ownerId: string;
  createdAt: number;
  documentTypesCount: number;
  totalDocuments: number;
}