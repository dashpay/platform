import { Platform } from "../../Platform";

/**
 * @param record - the exact name of the record to resolve
 * @param value - the exact value for this record to resolve
 * @returns {Document} document
 */
export async function resolveByRecord(this: Platform, record: string, value: any): Promise<any> {
    const [document] = await this.documents.get('dpns.domain', {
        where: [
            [`records.${record}`, '==', value],
        ],
    });

    return document;
}

export default resolveByRecord;
