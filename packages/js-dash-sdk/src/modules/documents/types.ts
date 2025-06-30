import { QueryOptions, WhereClause } from '../../core/types';

export interface Document {
  id: string;
  dataContractId: string;
  type: string;
  ownerId: string;
  revision: number;
  data: Record<string, any>;
  createdAt?: number;
  updatedAt?: number;
}

export interface DocumentCreateOptions {
  create: Array<{
    type: string;
    data: Record<string, any>;
  }>;
}

export interface DocumentReplaceOptions {
  replace: Array<{
    id: string;
    type: string;
    data: Record<string, any>;
    revision: number;
  }>;
}

export interface DocumentDeleteOptions {
  delete: Array<{
    id: string;
    type: string;
  }>;
}

export type DocumentsBatchOptions = DocumentCreateOptions | DocumentReplaceOptions | DocumentDeleteOptions | 
  (DocumentCreateOptions & DocumentReplaceOptions) |
  (DocumentCreateOptions & DocumentDeleteOptions) |
  (DocumentReplaceOptions & DocumentDeleteOptions) |
  (DocumentCreateOptions & DocumentReplaceOptions & DocumentDeleteOptions);

export interface DocumentQuery extends QueryOptions {
  dataContractId: string;
  type: string;
}