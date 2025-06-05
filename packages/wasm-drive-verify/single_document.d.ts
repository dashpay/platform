/**
 * WASM bindings for SingleDocumentDriveQuery verification
 */

/**
 * Contested status for a single document query
 */
export enum SingleDocumentDriveQueryContestedStatus {
  NotContested = 0,
  MaybeContested = 1,
  Contested = 2,
}

/**
 * WASM wrapper for SingleDocumentDriveQuery
 */
export class SingleDocumentDriveQueryWasm {
  /**
   * Create a new SingleDocumentDriveQuery
   * @param contractId - The contract ID (must be exactly 32 bytes)
   * @param documentTypeName - The name of the document type
   * @param documentTypeKeepsHistory - Whether the document type keeps history
   * @param documentId - The document ID (must be exactly 32 bytes)
   * @param blockTimeMs - Optional block time in milliseconds
   * @param contestedStatus - The contested status (0, 1, or 2)
   */
  constructor(
    contractId: Uint8Array,
    documentTypeName: string,
    documentTypeKeepsHistory: boolean,
    documentId: Uint8Array,
    blockTimeMs?: number,
    contestedStatus?: number
  );

  /**
   * Get the contract ID
   */
  readonly contractId: Uint8Array;

  /**
   * Get the document type name
   */
  readonly documentTypeName: string;

  /**
   * Get whether the document type keeps history
   */
  readonly documentTypeKeepsHistory: boolean;

  /**
   * Get the document ID
   */
  readonly documentId: Uint8Array;

  /**
   * Get the block time in milliseconds
   */
  readonly blockTimeMs?: number;

  /**
   * Get the contested status
   */
  readonly contestedStatus: number;
}

/**
 * Result of a single document proof verification
 */
export class SingleDocumentProofResult {
  /**
   * Get the root hash
   */
  readonly rootHash: Uint8Array;

  /**
   * Get the serialized document (if found)
   */
  readonly documentSerialized?: Uint8Array;

  /**
   * Check if a document was found
   */
  hasDocument(): boolean;
}

/**
 * Verify a single document proof and keep it serialized
 * @param query - The query to verify
 * @param isSubset - Whether to verify a subset of a larger proof
 * @param proof - The proof to verify
 * @returns The verification result
 */
export function verifySingleDocumentProofKeepSerialized(
  query: SingleDocumentDriveQueryWasm,
  isSubset: boolean,
  proof: Uint8Array
): SingleDocumentProofResult;

/**
 * Create a SingleDocumentDriveQuery for a non-contested document
 * @param contractId - The contract ID (must be exactly 32 bytes)
 * @param documentTypeName - The name of the document type
 * @param documentTypeKeepsHistory - Whether the document type keeps history
 * @param documentId - The document ID (must be exactly 32 bytes)
 * @param blockTimeMs - Optional block time in milliseconds
 * @returns The created query
 */
export function createSingleDocumentQuery(
  contractId: Uint8Array,
  documentTypeName: string,
  documentTypeKeepsHistory: boolean,
  documentId: Uint8Array,
  blockTimeMs?: number
): SingleDocumentDriveQueryWasm;

/**
 * Create a SingleDocumentDriveQuery for a maybe contested document
 * @param contractId - The contract ID (must be exactly 32 bytes)
 * @param documentTypeName - The name of the document type
 * @param documentTypeKeepsHistory - Whether the document type keeps history
 * @param documentId - The document ID (must be exactly 32 bytes)
 * @param blockTimeMs - Optional block time in milliseconds
 * @returns The created query
 */
export function createSingleDocumentQueryMaybeContested(
  contractId: Uint8Array,
  documentTypeName: string,
  documentTypeKeepsHistory: boolean,
  documentId: Uint8Array,
  blockTimeMs?: number
): SingleDocumentDriveQueryWasm;

/**
 * Create a SingleDocumentDriveQuery for a contested document
 * @param contractId - The contract ID (must be exactly 32 bytes)
 * @param documentTypeName - The name of the document type
 * @param documentTypeKeepsHistory - Whether the document type keeps history
 * @param documentId - The document ID (must be exactly 32 bytes)
 * @param blockTimeMs - Optional block time in milliseconds
 * @returns The created query
 */
export function createSingleDocumentQueryContested(
  contractId: Uint8Array,
  documentTypeName: string,
  documentTypeKeepsHistory: boolean,
  documentId: Uint8Array,
  blockTimeMs?: number
): SingleDocumentDriveQueryWasm;