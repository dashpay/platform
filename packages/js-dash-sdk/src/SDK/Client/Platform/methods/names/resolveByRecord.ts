import { Identifier } from '@dashevo/wasm-dpp';
import { Platform } from '../../Platform';

/**
 * @param record - the exact name of the record to resolve
 * @param value - the exact value for this record to resolve
 * @returns {ExtendedDocument[]} - Resolved domains
 */
export async function resolveByRecord(this: Platform, record: string, value: any): Promise<any> {
  await this.initialize();

  // If wasm-sdk is available, delegate to it
  if (this.wasmSdk && this.getAdapter()) {
    const adapter = this.getAdapter()!;
    
    // Convert identity values to string format
    let recordValue = value;
    if (record === 'identity') {
      recordValue = Identifier.from(value).toString();
    } else if (typeof value === 'object' && value.toString) {
      recordValue = value.toString();
    }
    
    this.logger.debug(`[Names#resolveByRecord] Calling wasm-sdk getNameByRecord for record "${record}" with value "${recordValue}"`);
    
    // Call wasm-sdk getNameByRecord
    const result = await this.wasmSdk.getNameByRecord(
      record,
      recordValue,
      undefined // parentDomainName is optional
    );
    
    if (!result) {
      return [];
    }
    
    // Convert the result to array of documents
    const documents = Array.isArray(result) ? result : [result];
    
    this.logger.debug(`[Names#resolveByRecord] Found ${documents.length} names via wasm-sdk`);
    
    return documents.map(doc => adapter.convertResponse(doc, 'document'));
  }

  if (record === 'identity') {
    value = Identifier.from(value);
  }

  return await this.documents.get('dpns.domain', {
    where: [
      [`records.${record}`, '==', value],
    ],
  });
}

export default resolveByRecord;
