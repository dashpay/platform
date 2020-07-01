import { Platform } from "../../Platform";

export async function resolveByRecord(this: Platform, record: string, value: any): Promise<any> {
    const [document] = await this.documents.get('dpns.domain', {
        where: [
            [`records.${record}`, '==', value],
        ],
    });

    return document;
}

export default resolveByRecord;
