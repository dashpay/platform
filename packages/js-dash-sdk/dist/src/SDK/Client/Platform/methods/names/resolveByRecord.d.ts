import { Platform } from '../../Platform';
/**
 * @param record - the exact name of the record to resolve
 * @param value - the exact value for this record to resolve
 * @returns {Document[]} - Resolved domains
 */
export declare function resolveByRecord(this: Platform, record: string, value: any): Promise<any>;
export default resolveByRecord;
