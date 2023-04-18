import { Identifier } from '@dashevo/wasm-dpp';
import { Platform } from '../../Platform';

/**
 * @param record - the exact name of the record to resolve
 * @param value - the exact value for this record to resolve
 * @returns {ExtendedDocument[]} - Resolved domains
 */
export async function resolveByRecord(this: Platform, record: string, value: any): Promise<any> {
  await this.initialize();

  if (record === 'dashUniqueIdentityId' || record === 'dashAliasIdentityId') {
    value = Identifier.from(value);
  }

  return await this.documents.get('dpns.domain', {
    where: [
      [`records.${record}`, '==', value],
    ],
  });
}

export default resolveByRecord;
