import { Platform } from "../../Platform";
import Identifier from "@dashevo/dpp/lib/Identifier";

/**
 * @param record - the exact name of the record to resolve
 * @param value - the exact value for this record to resolve
 * @returns {Document[]} - Resolved domains
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
