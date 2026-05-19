import wasmDpp from '@dashevo/wasm-dpp';
const { Identifier, ExtendedDocument } = wasmDpp;
import type {
  Identifier as IdentifierType,
  ExtendedDocument as ExtendedDocumentType,
} from '@dashevo/wasm-dpp';
type Identifier = IdentifierType;
type ExtendedDocument = ExtendedDocumentType;

/**
 * @param {WhereCondition[]} [where] - where
 * @param {OrderByCondition[]} [orderBy] - order by
 * @param {number} [limit] - limit
 * @param {string|Buffer|ExtendedDocument|Identifier} [startAt] - start value (included)
 * @param {string|Buffer|ExtendedDocument|Identifier} [startAfter] - start value (not included)
 */
export type QueryOptions = {
  where?: WhereCondition[];
  orderBy?: OrderByCondition[];
  limit?: number;
  startAt?: string | Buffer | ExtendedDocument | Identifier;
  startAfter?: string | Buffer | ExtendedDocument | Identifier;
};

export type OrderByCondition = [
  string,
  'asc' | 'desc',
];

export type WhereCondition = [
  string,
  '<' | '<=' | '==' | '>' | '>=' | 'in' | 'startsWith' | 'elementMatch' | 'length' | 'contains',
  WhereCondition | any,
];
