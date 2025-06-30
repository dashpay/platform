/**
 * Shared contract validation utilities to avoid circular dependencies
 */

import { SDK } from '../../SDK';
import { getWasmSdk } from '../../core/WasmLoader';

export interface ContractInfo {
  id: string;
  documentSchemas: Record<string, any>;
}

/**
 * Validates a contract exists and returns basic info
 */
export async function validateContract(
  sdk: SDK,
  dataContractId: string
): Promise<ContractInfo> {
  const wasm = getWasmSdk();
  const wasmSdk = sdk.getWasmSdk();
  
  try {
    const contractResult = await wasm.fetchDataContract(wasmSdk, dataContractId);
    
    if (!contractResult) {
      throw new Error(`Data contract ${dataContractId} not found`);
    }
    
    return {
      id: contractResult.id,
      documentSchemas: contractResult.documentSchemas || {}
    };
  } catch (error: any) {
    if (error.message?.includes('not found')) {
      throw new Error(`Data contract ${dataContractId} not found`);
    }
    throw error;
  }
}

/**
 * Validates a document type exists in a contract
 */
export function validateDocumentType(
  contract: ContractInfo,
  documentType: string
): void {
  if (!contract.documentSchemas[documentType]) {
    throw new Error(`Document type '${documentType}' not found in contract ${contract.id}`);
  }
}